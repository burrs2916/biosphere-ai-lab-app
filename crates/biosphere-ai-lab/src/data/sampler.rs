use rand::prelude::*;
use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedSamplerConfig {
    pub num_replicas: usize,
    pub rank: usize,
    pub shuffle: bool,
    pub seed: u64,
    pub drop_last: bool,
}

impl Default for DistributedSamplerConfig {
    fn default() -> Self {
        Self {
            num_replicas: 1,
            rank: 0,
            shuffle: true,
            seed: 42,
            drop_last: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedRandomSamplerConfig {
    pub num_samples: usize,
    pub replacement: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSamplerConfig {
    pub batch_size: usize,
    pub drop_last: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SamplerType {
    Sequential,
    Random { seed: u64 },
    Distributed(DistributedSamplerConfig),
    WeightedRandom(WeightedRandomSamplerConfig),
    SubsetRandom { indices: Vec<usize>, seed: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplerStats {
    pub total_samples: usize,
    pub samples_yielded: usize,
    pub batches_yielded: usize,
    pub epoch: usize,
    pub rank: usize,
    pub num_replicas: usize,
}

pub struct DistributedSampler {
    config: DistributedSamplerConfig,
    indices: Vec<usize>,
    epoch: usize,
    rng: StdRng,
}

impl DistributedSampler {
    pub fn new(total_size: usize, config: DistributedSamplerConfig) -> Self {
        let mut indices: Vec<usize> = (0..total_size).collect();

        let num_samples = if config.drop_last {
            total_size - (total_size % config.num_replicas)
        } else {
            total_size
        };

        let per_replica = num_samples / config.num_replicas;
        let remainder = num_samples % config.num_replicas;

        let start = config.rank * per_replica + remainder.min(config.rank);
        let end = start + per_replica + if config.rank < remainder { 1 } else { 0 };

        indices = indices[start..end].to_vec();

        let rng = StdRng::seed_from_u64(config.seed);

        Self {
            config,
            indices,
            epoch: 0,
            rng,
        }
    }

    pub fn set_epoch(&mut self, epoch: usize) {
        self.epoch = epoch;
        let seed = self.config.seed.wrapping_add(epoch as u64);
        self.rng = StdRng::seed_from_u64(seed);

        if self.config.shuffle {
            self.indices.shuffle(&mut self.rng);
        }
    }

    pub fn len(&self) -> usize {
        self.indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn iter(&self) -> DistributedSamplerIter {
        DistributedSamplerIter {
            sampler: self,
            position: 0,
        }
    }

    pub fn stats(&self) -> SamplerStats {
        SamplerStats {
            total_samples: self.indices.len(),
            samples_yielded: 0,
            batches_yielded: 0,
            epoch: self.epoch,
            rank: self.config.rank,
            num_replicas: self.config.num_replicas,
        }
    }
}

pub struct DistributedSamplerIter<'a> {
    sampler: &'a DistributedSampler,
    position: usize,
}

impl<'a> Iterator for DistributedSamplerIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.sampler.indices.len() {
            return None;
        }
        let idx = self.sampler.indices[self.position];
        self.position += 1;
        Some(idx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.sampler.indices.len() - self.position;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for DistributedSamplerIter<'a> {}

pub struct WeightedRandomSampler {
    weights: Vec<f64>,
    num_samples: usize,
    replacement: bool,
    rng: StdRng,
    total_weight: f64,
}

impl WeightedRandomSampler {
    pub fn new(
        weights: Vec<f64>,
        num_samples: usize,
        replacement: bool,
        seed: u64,
    ) -> Self {
        let total_weight: f64 = weights.iter().sum();
        let rng = StdRng::seed_from_u64(seed);

        Self {
            weights,
            num_samples,
            replacement,
            rng,
            total_weight,
        }
    }

    pub fn len(&self) -> usize {
        self.num_samples
    }

    pub fn is_empty(&self) -> bool {
        self.num_samples == 0
    }

    pub fn iter(&mut self) -> WeightedRandomSamplerIter {
        WeightedRandomSamplerIter {
            sampler: self,
            yielded: 0,
        }
    }
}

pub struct WeightedRandomSamplerIter<'a> {
    sampler: &'a mut WeightedRandomSampler,
    yielded: usize,
}

impl<'a> Iterator for WeightedRandomSamplerIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.yielded >= self.sampler.num_samples {
            return None;
        }

        if self.sampler.replacement {
            let r: f64 = self.sampler.rng.gen_range(0.0..1.0) * self.sampler.total_weight;
            let mut cumulative = 0.0;
            for (i, &w) in self.sampler.weights.iter().enumerate() {
                cumulative += w;
                if r < cumulative {
                    self.yielded += 1;
                    return Some(i);
                }
            }
            self.yielded += 1;
            Some(self.sampler.weights.len() - 1)
        } else {
            let r: f64 = self.sampler.rng.gen_range(0.0..1.0) * self.sampler.total_weight;
            let mut cumulative = 0.0;
            for (i, &w) in self.sampler.weights.iter().enumerate() {
                cumulative += w;
                if r < cumulative {
                    self.sampler.total_weight -= w;
                    self.sampler.weights[i] = 0.0;
                    self.yielded += 1;
                    return Some(i);
                }
            }
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.sampler.num_samples - self.yielded;
        (remaining, Some(remaining))
    }
}

pub struct BatchSampler {
    indices: Vec<usize>,
    batch_size: usize,
    drop_last: bool,
}

impl BatchSampler {
    pub fn new(indices: Vec<usize>, batch_size: usize, drop_last: bool) -> Self {
        Self {
            indices,
            batch_size,
            drop_last,
        }
    }

    pub fn len(&self) -> usize {
        if self.drop_last {
            self.indices.len() / self.batch_size
        } else {
            (self.indices.len() + self.batch_size - 1) / self.batch_size
        }
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn iter(&self) -> BatchSamplerIter {
        BatchSamplerIter {
            sampler: self,
            position: 0,
        }
    }
}

pub struct BatchSamplerIter<'a> {
    sampler: &'a BatchSampler,
    position: usize,
}

impl<'a> Iterator for BatchSamplerIter<'a> {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.sampler.indices.len() {
            return None;
        }

        let end = (self.position + self.sampler.batch_size).min(self.sampler.indices.len());

        if self.sampler.drop_last && (end - self.position) < self.sampler.batch_size {
            return None;
        }

        let batch: Vec<usize> = self.sampler.indices[self.position..end].to_vec();
        self.position = end;
        Some(batch)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = if self.sampler.drop_last {
            (self.sampler.indices.len() - self.position) / self.sampler.batch_size
        } else {
            let rem = self.sampler.indices.len().saturating_sub(self.position);
            if rem == 0 {
                0
            } else {
                (rem + self.sampler.batch_size - 1) / self.sampler.batch_size
            }
        };
        (remaining, Some(remaining))
    }
}

pub struct SubsetRandomSampler {
    indices: Vec<usize>,
    rng: StdRng,
}

impl SubsetRandomSampler {
    pub fn new(indices: Vec<usize>, seed: u64) -> Self {
        let rng = StdRng::seed_from_u64(seed);
        Self { indices, rng }
    }

    pub fn len(&self) -> usize {
        self.indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn iter(&mut self) -> SubsetRandomSamplerIter {
        self.indices.shuffle(&mut self.rng);
        SubsetRandomSamplerIter {
            sampler: self,
            position: 0,
        }
    }
}

pub struct SubsetRandomSamplerIter<'a> {
    sampler: &'a SubsetRandomSampler,
    position: usize,
}

impl<'a> Iterator for SubsetRandomSamplerIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.sampler.indices.len() {
            return None;
        }
        let idx = self.sampler.indices[self.position];
        self.position += 1;
        Some(idx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.sampler.indices.len() - self.position;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for SubsetRandomSamplerIter<'a> {}

pub fn compute_split_indices(
    total_size: usize,
    train_ratio: f64,
    val_ratio: f64,
    test_ratio: f64,
    seed: u64,
) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut indices: Vec<usize> = (0..total_size).collect();
    indices.shuffle(&mut rng);

    let train_end = (total_size as f64 * train_ratio) as usize;
    let val_end = train_end + (total_size as f64 * val_ratio) as usize;

    let train = indices[..train_end].to_vec();
    let val = indices[train_end..val_end].to_vec();
    let test = indices[val_end..].to_vec();

    (train, val, test)
}

pub fn compute_kfold_indices(
    total_size: usize,
    k: usize,
    seed: u64,
) -> Vec<(Vec<usize>, Vec<usize>)> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut indices: Vec<usize> = (0..total_size).collect();
    indices.shuffle(&mut rng);

    let fold_size = total_size / k;
    let mut folds = Vec::with_capacity(k);

    for i in 0..k {
        let val_start = i * fold_size;
        let val_end = if i == k - 1 { total_size } else { (i + 1) * fold_size };

        let val_indices: Vec<usize> = indices[val_start..val_end].to_vec();
        let train_indices: Vec<usize> = indices[..val_start]
            .iter()
            .chain(indices[val_end..].iter())
            .copied()
            .collect();

        folds.push((train_indices, val_indices));
    }

    folds
}

pub fn compute_stratified_split_indices(
    labels: &[usize],
    num_classes: usize,
    train_ratio: f64,
    val_ratio: f64,
    test_ratio: f64,
    seed: u64,
) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    let mut rng = StdRng::seed_from_u64(seed);

    let mut class_indices: Vec<Vec<usize>> = vec![Vec::new(); num_classes];
    for (i, &label) in labels.iter().enumerate() {
        if label < num_classes {
            class_indices[label].push(i);
        }
    }

    let mut train = Vec::new();
    let mut val = Vec::new();
    let mut test = Vec::new();

    for class_idx in 0..num_classes {
        let mut indices = class_indices[class_idx].clone();
        indices.shuffle(&mut rng);

        let n = indices.len();
        let train_end = (n as f64 * train_ratio) as usize;
        let val_end = train_end + (n as f64 * val_ratio) as usize;

        train.extend_from_slice(&indices[..train_end]);
        val.extend_from_slice(&indices[train_end..val_end]);
        test.extend_from_slice(&indices[val_end..]);
    }

    train.shuffle(&mut rng);
    val.shuffle(&mut rng);
    test.shuffle(&mut rng);

    (train, val, test)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distributed_sampler_single_replica() {
        let config = DistributedSamplerConfig {
            num_replicas: 1,
            rank: 0,
            shuffle: false,
            seed: 42,
            drop_last: false,
        };
        let sampler = DistributedSampler::new(100, config);
        assert_eq!(sampler.len(), 100);

        let indices: Vec<usize> = sampler.iter().collect();
        assert_eq!(indices.len(), 100);
        assert_eq!(indices[0], 0);
        assert_eq!(indices[99], 99);
    }

    #[test]
    fn test_distributed_sampler_multi_replica() {
        let config0 = DistributedSamplerConfig {
            num_replicas: 4,
            rank: 0,
            shuffle: false,
            seed: 42,
            drop_last: false,
        };
        let config1 = DistributedSamplerConfig {
            num_replicas: 4,
            rank: 1,
            shuffle: false,
            seed: 42,
            drop_last: false,
        };
        let config2 = DistributedSamplerConfig {
            num_replicas: 4,
            rank: 2,
            shuffle: false,
            seed: 42,
            drop_last: false,
        };
        let config3 = DistributedSamplerConfig {
            num_replicas: 4,
            rank: 3,
            shuffle: false,
            seed: 42,
            drop_last: false,
        };

        let s0 = DistributedSampler::new(100, config0);
        let s1 = DistributedSampler::new(100, config1);
        let s2 = DistributedSampler::new(100, config2);
        let s3 = DistributedSampler::new(100, config3);

        let i0: Vec<usize> = s0.iter().collect();
        let i1: Vec<usize> = s1.iter().collect();
        let i2: Vec<usize> = s2.iter().collect();
        let i3: Vec<usize> = s3.iter().collect();

        assert_eq!(i0.len(), 25);
        assert_eq!(i1.len(), 25);
        assert_eq!(i2.len(), 25);
        assert_eq!(i3.len(), 25);

        let all: HashSet<usize> = i0.iter().chain(i1.iter()).chain(i2.iter()).chain(i3.iter()).copied().collect();
        assert_eq!(all.len(), 100);
    }

    #[test]
    fn test_distributed_sampler_drop_last() {
        let config = DistributedSamplerConfig {
            num_replicas: 3,
            rank: 0,
            shuffle: false,
            seed: 42,
            drop_last: true,
        };
        let sampler = DistributedSampler::new(100, config);
        assert_eq!(sampler.len(), 33);
    }

    #[test]
    fn test_distributed_sampler_shuffle() {
        let config = DistributedSamplerConfig {
            num_replicas: 1,
            rank: 0,
            shuffle: true,
            seed: 42,
            drop_last: false,
        };
        let mut sampler = DistributedSampler::new(100, config);
        sampler.set_epoch(0);
        let epoch0: Vec<usize> = sampler.iter().collect();

        sampler.set_epoch(1);
        let epoch1: Vec<usize> = sampler.iter().collect();

        assert_ne!(epoch0, epoch1);
        assert_eq!(epoch0.len(), 100);
        assert_eq!(epoch1.len(), 100);
    }

    #[test]
    fn test_weighted_random_sampler() {
        let weights = vec![1.0, 2.0, 3.0, 4.0];
        let mut sampler = WeightedRandomSampler::new(weights, 100, true, 42);
        let indices: Vec<usize> = sampler.iter().collect();
        assert_eq!(indices.len(), 100);

        let count_3: usize = indices.iter().filter(|&&i| i == 3).count();
        let count_0: usize = indices.iter().filter(|&&i| i == 0).count();
        assert!(count_3 > count_0);
    }

    #[test]
    fn test_batch_sampler() {
        let indices: Vec<usize> = (0..100).collect();
        let sampler = BatchSampler::new(indices, 32, false);
        assert_eq!(sampler.len(), 4);

        let batches: Vec<Vec<usize>> = sampler.iter().collect();
        assert_eq!(batches.len(), 4);
        assert_eq!(batches[0].len(), 32);
        assert_eq!(batches[3].len(), 4);
    }

    #[test]
    fn test_batch_sampler_drop_last() {
        let indices: Vec<usize> = (0..100).collect();
        let sampler = BatchSampler::new(indices, 32, true);
        assert_eq!(sampler.len(), 3);

        let batches: Vec<Vec<usize>> = sampler.iter().collect();
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].len(), 32);
        assert_eq!(batches[2].len(), 32);
    }

    #[test]
    fn test_subset_random_sampler() {
        let indices: Vec<usize> = (0..100).collect();
        let mut sampler = SubsetRandomSampler::new(indices, 42);
        let result: Vec<usize> = sampler.iter().collect();
        assert_eq!(result.len(), 100);

        let sorted: Vec<usize> = {
            let mut s = result.clone();
            s.sort();
            s
        };
        assert_eq!(sorted, (0..100).collect::<Vec<usize>>());
    }

    #[test]
    fn test_compute_split_indices() {
        let (train, val, test) = compute_split_indices(1000, 0.7, 0.15, 0.15, 42);
        assert_eq!(train.len(), 700);
        assert_eq!(val.len(), 150);
        assert_eq!(test.len(), 150);

        let all: HashSet<usize> = train.iter().chain(val.iter()).chain(test.iter()).copied().collect();
        assert_eq!(all.len(), 1000);
    }

    #[test]
    fn test_compute_kfold_indices() {
        let folds = compute_kfold_indices(100, 5, 42);
        assert_eq!(folds.len(), 5);

        for (train, val) in &folds {
            assert_eq!(val.len(), 20);
            assert_eq!(train.len(), 80);
            let combined: HashSet<usize> = train.iter().chain(val.iter()).copied().collect();
            assert_eq!(combined.len(), 100);
        }
    }

    #[test]
    fn test_stratified_split() {
        let labels: Vec<usize> = vec![0, 0, 0, 0, 0, 1, 1, 1, 1, 1];
        let (train, val, test) = compute_stratified_split_indices(&labels, 2, 0.6, 0.2, 0.2, 42);

        let train_labels: Vec<usize> = train.iter().map(|&i| labels[i]).collect();
        let val_labels: Vec<usize> = val.iter().map(|&i| labels[i]).collect();
        let test_labels: Vec<usize> = test.iter().map(|&i| labels[i]).collect();

        assert_eq!(train_labels.iter().filter(|&&l| l == 0).count(), 3);
        assert_eq!(train_labels.iter().filter(|&&l| l == 1).count(), 3);
        assert_eq!(val_labels.iter().filter(|&&l| l == 0).count(), 1);
        assert_eq!(val_labels.iter().filter(|&&l| l == 1).count(), 1);
        assert_eq!(test_labels.iter().filter(|&&l| l == 0).count(), 1);
        assert_eq!(test_labels.iter().filter(|&&l| l == 1).count(), 1);
    }
}
