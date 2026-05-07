use rand::prelude::*;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalShuffleConfig {
    pub num_workers: usize,
    pub worker_rank: usize,
    pub seed: u64,
    pub shuffle_across_epochs: bool,
    pub buffer_size: usize,
    pub drop_last: bool,
    pub elastic: bool,
}

impl Default for GlobalShuffleConfig {
    fn default() -> Self {
        Self {
            num_workers: 1,
            worker_rank: 0,
            seed: 42,
            shuffle_across_epochs: true,
            buffer_size: 10000,
            drop_last: false,
            elastic: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardManifest {
    pub shards: Vec<ShardInfo>,
    pub total_samples: usize,
    pub total_size_bytes: u64,
    pub format: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardInfo {
    pub shard_id: usize,
    pub path: String,
    pub num_samples: usize,
    pub size_bytes: u64,
    pub checksum: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ShardManifest {
    pub fn new(shards: Vec<ShardInfo>, format: &str) -> Self {
        let total_samples: usize = shards.iter().map(|s| s.num_samples).sum();
        let total_size_bytes: u64 = shards.iter().map(|s| s.size_bytes).sum();

        Self {
            shards,
            total_samples,
            total_size_bytes,
            format: format.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn sample_to_shard(&self, global_index: usize) -> Option<(usize, usize)> {
        let mut offset = 0usize;
        for shard in &self.shards {
            if global_index < offset + shard.num_samples {
                return Some((shard.shard_id, global_index - offset));
            }
            offset += shard.num_samples;
        }
        None
    }

    pub fn shard_sample_range(&self, shard_id: usize) -> Option<(usize, usize)> {
        let mut offset = 0usize;
        for shard in &self.shards {
            if shard.shard_id == shard_id {
                return Some((offset, offset + shard.num_samples));
            }
            offset += shard.num_samples;
        }
        None
    }
}

pub struct GlobalShuffleSampler {
    config: GlobalShuffleConfig,
    total_samples: usize,
    permutation: Vec<usize>,
    epoch: usize,
    position: usize,
    rng: StdRng,
}

impl GlobalShuffleSampler {
    pub fn new(total_samples: usize, config: GlobalShuffleConfig) -> Self {
        let mut sampler = Self {
            config,
            total_samples,
            permutation: Vec::new(),
            epoch: 0,
            position: 0,
            rng: StdRng::seed_from_u64(0),
        };
        sampler.build_permutation();
        sampler
    }

    fn build_permutation(&mut self) {
        let epoch_seed = if self.config.shuffle_across_epochs {
            self.config.seed.wrapping_add(self.epoch as u64)
        } else {
            self.config.seed
        };
        self.rng = StdRng::seed_from_u64(epoch_seed);

        let num_samples = if self.config.drop_last {
            self.total_samples - (self.total_samples % self.config.num_workers)
        } else {
            self.total_samples
        };

        let per_worker = num_samples / self.config.num_workers;
        let remainder = num_samples % self.config.num_workers;

        let start = self.config.worker_rank * per_worker + remainder.min(self.config.worker_rank);
        let end = start + per_worker + if self.config.worker_rank < remainder { 1 } else { 0 };

        self.permutation = (start..end).collect();
        self.permutation.shuffle(&mut self.rng);
        self.position = 0;
    }

    pub fn set_epoch(&mut self, epoch: usize) {
        self.epoch = epoch;
        self.build_permutation();
    }

    pub fn len(&self) -> usize {
        self.permutation.len()
    }

    pub fn is_empty(&self) -> bool {
        self.permutation.is_empty()
    }

    pub fn iter(&self) -> GlobalShuffleIter {
        GlobalShuffleIter {
            sampler: self,
            position: 0,
        }
    }

    pub fn stats(&self) -> GlobalShuffleStats {
        GlobalShuffleStats {
            total_samples: self.total_samples,
            worker_samples: self.permutation.len(),
            epoch: self.epoch,
            worker_rank: self.config.worker_rank,
            num_workers: self.config.num_workers,
            seed: self.config.seed,
            elastic: self.config.elastic,
        }
    }
}

pub struct GlobalShuffleIter<'a> {
    sampler: &'a GlobalShuffleSampler,
    position: usize,
}

impl<'a> Iterator for GlobalShuffleIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.sampler.permutation.len() {
            return None;
        }
        let idx = self.sampler.permutation[self.position];
        self.position += 1;
        Some(idx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.sampler.permutation.len() - self.position;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for GlobalShuffleIter<'a> {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalShuffleStats {
    pub total_samples: usize,
    pub worker_samples: usize,
    pub epoch: usize,
    pub worker_rank: usize,
    pub num_workers: usize,
    pub seed: u64,
    pub elastic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElasticShardConfig {
    pub num_workers: usize,
    pub worker_rank: usize,
    pub prefetch_shards: usize,
    pub cache_dir: Option<String>,
    pub max_cached_shards: usize,
    pub seed: u64,
    pub timeout_ms: u64,
}

impl Default for ElasticShardConfig {
    fn default() -> Self {
        Self {
            num_workers: 1,
            worker_rank: 0,
            prefetch_shards: 2,
            cache_dir: None,
            max_cached_shards: 32,
            seed: 42,
            timeout_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShardState {
    Available,
    Claimed(usize),
    Processing(usize),
    Completed(usize),
    Failed(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardAllocation {
    pub shard_id: usize,
    pub worker_rank: usize,
    pub state: ShardState,
    pub claimed_at: String,
    pub completed_at: Option<String>,
    pub num_samples: usize,
    pub size_bytes: u64,
}

pub struct ElasticShardAllocator {
    config: ElasticShardConfig,
    manifest: ShardManifest,
    allocations: Arc<Mutex<HashMap<usize, ShardAllocation>>>,
    worker_heartbeats: Arc<Mutex<HashMap<usize, Instant>>>,
    is_active: Arc<AtomicBool>,
    rng: StdRng,
}

impl ElasticShardAllocator {
    pub fn new(manifest: ShardManifest, config: ElasticShardConfig) -> Self {
        let rng = StdRng::seed_from_u64(config.seed);

        let mut allocations = HashMap::new();
        for shard in &manifest.shards {
            allocations.insert(
                shard.shard_id,
                ShardAllocation {
                    shard_id: shard.shard_id,
                    worker_rank: 0,
                    state: ShardState::Available,
                    claimed_at: String::new(),
                    completed_at: None,
                    num_samples: shard.num_samples,
                    size_bytes: shard.size_bytes,
                },
            );
        }

        Self {
            config,
            manifest,
            allocations: Arc::new(Mutex::new(allocations)),
            worker_heartbeats: Arc::new(Mutex::new(HashMap::new())),
            is_active: Arc::new(AtomicBool::new(true)),
            rng,
        }
    }

    pub fn claim_shard(&mut self) -> Option<ShardInfo> {
        let mut allocs = self.allocations.lock().unwrap();

        let available: Vec<usize> = allocs.iter()
            .filter(|(_, a)| matches!(a.state, ShardState::Available))
            .map(|(&id, _)| id)
            .collect();

        if available.is_empty() {
            return None;
        }

        let idx = self.rng.gen_range(0..available.len());
        let shard_id = available[idx];

        if let Some(alloc) = allocs.get_mut(&shard_id) {
            alloc.worker_rank = self.config.worker_rank;
            alloc.state = ShardState::Claimed(self.config.worker_rank);
            alloc.claimed_at = chrono::Utc::now().to_rfc3339();
        }

        self.manifest.shards.iter()
            .find(|s| s.shard_id == shard_id)
            .cloned()
    }

    pub fn mark_processing(&self, shard_id: usize) {
        if let Ok(mut allocs) = self.allocations.lock() {
            if let Some(alloc) = allocs.get_mut(&shard_id) {
                alloc.state = ShardState::Processing(self.config.worker_rank);
            }
        }
    }

    pub fn mark_completed(&self, shard_id: usize) {
        if let Ok(mut allocs) = self.allocations.lock() {
            if let Some(alloc) = allocs.get_mut(&shard_id) {
                alloc.state = ShardState::Completed(self.config.worker_rank);
                alloc.completed_at = Some(chrono::Utc::now().to_rfc3339());
            }
        }
    }

    pub fn mark_failed(&self, shard_id: usize) {
        if let Ok(mut allocs) = self.allocations.lock() {
            if let Some(alloc) = allocs.get_mut(&shard_id) {
                alloc.state = ShardState::Failed(self.config.worker_rank);
            }
        }
    }

    pub fn heartbeat(&self) {
        if let Ok(mut beats) = self.worker_heartbeats.lock() {
            beats.insert(self.config.worker_rank, Instant::now());
        }
    }

    pub fn reclaim_stale_shards(&mut self) -> Vec<usize> {
        let mut reclaimed = Vec::new();
        let timeout = Duration::from_millis(self.config.timeout_ms);

        if let Ok(beats) = self.worker_heartbeats.lock() {
            let now = Instant::now();
            let stale_workers: Vec<usize> = beats.iter()
                .filter(|(_, &last)| now.duration_since(last) > timeout)
                .map(|(&w, _)| w)
                .collect();

            if let Ok(mut allocs) = self.allocations.lock() {
                for worker in stale_workers {
                    for (shard_id, alloc) in allocs.iter_mut() {
                        if matches!(alloc.state, ShardState::Claimed(w) | ShardState::Processing(w) if w == worker)
                        {
                            alloc.state = ShardState::Available;
                            alloc.worker_rank = 0;
                            reclaimed.push(*shard_id);
                        }
                    }
                }
            }
        }

        reclaimed
    }

    pub fn progress(&self) -> ShardProgress {
        if let Ok(allocs) = self.allocations.lock() {
            let total = allocs.len();
            let completed = allocs.values()
                .filter(|a| matches!(a.state, ShardState::Completed(_)))
                .count();
            let in_progress = allocs.values()
                .filter(|a| matches!(a.state, ShardState::Claimed(_) | ShardState::Processing(_)))
                .count();
            let failed = allocs.values()
                .filter(|a| matches!(a.state, ShardState::Failed(_)))
                .count();
            let available = total - completed - in_progress - failed;

            ShardProgress {
                total_shards: total,
                completed,
                in_progress,
                failed,
                available,
                completion_pct: if total > 0 {
                    (completed as f64 / total as f64) * 100.0
                } else {
                    0.0
                },
            }
        } else {
            ShardProgress::default()
        }
    }

    pub fn shutdown(&self) {
        self.is_active.store(false, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardProgress {
    pub total_shards: usize,
    pub completed: usize,
    pub in_progress: usize,
    pub failed: usize,
    pub available: usize,
    pub completion_pct: f64,
}

impl Default for ShardProgress {
    fn default() -> Self {
        Self {
            total_shards: 0,
            completed: 0,
            in_progress: 0,
            failed: 0,
            available: 0,
            completion_pct: 0.0,
        }
    }
}

pub struct StreamingGlobalShuffle {
    config: GlobalShuffleConfig,
    manifest: ShardManifest,
    allocator: ElasticShardAllocator,
    current_shard: Option<ShardInfo>,
    shard_sample_offset: usize,
    buffer: VecDeque<usize>,
    epoch: usize,
    rng: StdRng,
}

impl StreamingGlobalShuffle {
    pub fn new(manifest: ShardManifest, config: GlobalShuffleConfig) -> Self {
        let elastic_config = ElasticShardConfig {
            num_workers: config.num_workers,
            worker_rank: config.worker_rank,
            seed: config.seed,
            ..Default::default()
        };
        let allocator = ElasticShardAllocator::new(manifest.clone(), elastic_config);

        let rng = StdRng::seed_from_u64(config.seed);
        let buffer_size = config.buffer_size;

        Self {
            config,
            manifest,
            allocator,
            current_shard: None,
            shard_sample_offset: 0,
            buffer: VecDeque::with_capacity(buffer_size),
            epoch: 0,
            rng,
        }
    }

    pub fn set_epoch(&mut self, epoch: usize) {
        self.epoch = epoch;
        let seed = self.config.seed.wrapping_add(epoch as u64);
        self.rng = StdRng::seed_from_u64(seed);
        self.buffer.clear();
        self.current_shard = None;
    }

    pub fn next_sample(&mut self) -> Option<usize> {
        if !self.buffer.is_empty() {
            return self.buffer.pop_front();
        }

        self.fill_buffer();
        self.buffer.pop_front()
    }

    fn fill_buffer(&mut self) {
        while self.buffer.len() < self.config.buffer_size {
            if self.current_shard.is_none() {
                match self.allocator.claim_shard() {
                    Some(shard) => {
                        self.allocator.mark_processing(shard.shard_id);
                        self.current_shard = Some(shard);
                        self.shard_sample_offset = 0;
                    }
                    None => {
                        let reclaimed = self.allocator.reclaim_stale_shards();
                        if reclaimed.is_empty() {
                            break;
                        }
                        continue;
                    }
                }
            }

            if let Some(ref shard) = self.current_shard {
                let remaining = shard.num_samples - self.shard_sample_offset;
                let to_take = remaining.min(self.config.buffer_size - self.buffer.len());

                let (global_start, _) = self.manifest.shard_sample_range(shard.shard_id)
                    .unwrap_or((0, 0));

                let mut indices: Vec<usize> = (self.shard_sample_offset..self.shard_sample_offset + to_take)
                    .map(|local| global_start + local)
                    .collect();

                if self.config.shuffle_across_epochs {
                    indices.shuffle(&mut self.rng);
                }

                self.buffer.extend(indices);
                self.shard_sample_offset += to_take;

                if self.shard_sample_offset >= shard.num_samples {
                    self.allocator.mark_completed(shard.shard_id);
                    self.current_shard = None;
                }
            }
        }
    }

    pub fn progress(&self) -> ShardProgress {
        self.allocator.progress()
    }

    pub fn stats(&self) -> GlobalShuffleStats {
        GlobalShuffleStats {
            total_samples: self.manifest.total_samples,
            worker_samples: 0,
            epoch: self.epoch,
            worker_rank: self.config.worker_rank,
            num_workers: self.config.num_workers,
            seed: self.config.seed,
            elastic: self.config.elastic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manifest(num_shards: usize, samples_per_shard: usize) -> ShardManifest {
        let shards: Vec<ShardInfo> = (0..num_shards)
            .map(|i| ShardInfo {
                shard_id: i,
                path: format!("shard_{}.parquet", i),
                num_samples: samples_per_shard,
                size_bytes: 1024 * 1024,
                checksum: None,
                metadata: HashMap::new(),
            })
            .collect();
        ShardManifest::new(shards, "parquet")
    }

    #[test]
    fn test_global_shuffle_deterministic() {
        let config = GlobalShuffleConfig {
            num_workers: 4,
            worker_rank: 0,
            seed: 42,
            shuffle_across_epochs: true,
            buffer_size: 1000,
            drop_last: false,
            elastic: false,
        };

        let sampler1 = GlobalShuffleSampler::new(1000, config.clone());
        let sampler2 = GlobalShuffleSampler::new(1000, config);

        let indices1: Vec<usize> = sampler1.iter().collect();
        let indices2: Vec<usize> = sampler2.iter().collect();

        assert_eq!(indices1, indices2);
        assert_eq!(indices1.len(), 250);
    }

    #[test]
    fn test_global_shuffle_different_ranks() {
        let config0 = GlobalShuffleConfig {
            num_workers: 4,
            worker_rank: 0,
            seed: 42,
            ..Default::default()
        };
        let config1 = GlobalShuffleConfig {
            num_workers: 4,
            worker_rank: 1,
            seed: 42,
            ..Default::default()
        };

        let sampler0 = GlobalShuffleSampler::new(1000, config0);
        let sampler1 = GlobalShuffleSampler::new(1000, config1);

        let indices0: HashSet<usize> = sampler0.iter().collect();
        let indices1: HashSet<usize> = sampler1.iter().collect();

        assert!(indices0.is_disjoint(&indices1));
    }

    #[test]
    fn test_global_shuffle_epoch_change() {
        let config = GlobalShuffleConfig {
            num_workers: 1,
            worker_rank: 0,
            seed: 42,
            ..Default::default()
        };

        let mut sampler = GlobalShuffleSampler::new(100, config);
        let epoch0: Vec<usize> = sampler.iter().collect();

        sampler.set_epoch(1);
        let epoch1: Vec<usize> = sampler.iter().collect();

        assert_ne!(epoch0, epoch1);
    }

    #[test]
    fn test_shard_manifest() {
        let manifest = create_test_manifest(5, 100);
        assert_eq!(manifest.total_samples, 500);
        assert_eq!(manifest.shards.len(), 5);

        let (shard_id, local_idx) = manifest.sample_to_shard(150).unwrap();
        assert_eq!(shard_id, 1);
        assert_eq!(local_idx, 50);

        let (start, end) = manifest.shard_sample_range(2).unwrap();
        assert_eq!(start, 200);
        assert_eq!(end, 300);
    }

    #[test]
    fn test_elastic_shard_allocator() {
        let manifest = create_test_manifest(10, 100);
        let config = ElasticShardConfig {
            num_workers: 2,
            worker_rank: 0,
            ..Default::default()
        };

        let mut allocator = ElasticShardAllocator::new(manifest, config);

        let shard = allocator.claim_shard();
        assert!(shard.is_some());

        let progress = allocator.progress();
        assert_eq!(progress.total_shards, 10);
        assert_eq!(progress.in_progress, 1);
    }

    #[test]
    fn test_streaming_global_shuffle() {
        let manifest = create_test_manifest(5, 100);
        let config = GlobalShuffleConfig {
            num_workers: 1,
            worker_rank: 0,
            seed: 42,
            buffer_size: 50,
            ..Default::default()
        };

        let mut shuffle = StreamingGlobalShuffle::new(manifest, config);

        let mut samples = Vec::new();
        for _ in 0..200 {
            if let Some(idx) = shuffle.next_sample() {
                samples.push(idx);
            }
        }

        assert_eq!(samples.len(), 200);
        let unique: HashSet<usize> = samples.iter().copied().collect();
        assert_eq!(unique.len(), 200);
    }
}
