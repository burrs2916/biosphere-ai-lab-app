use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalDedupConfig {
    pub num_permutations: usize,
    pub num_bands: usize,
    pub rows_per_band: usize,
    pub similarity_threshold: f64,
    pub shingle_size: usize,
    pub min_document_length: usize,
    pub max_document_length: usize,
    pub use_exact_dedup: bool,
    pub use_near_dedup: bool,
    pub use_fuzzy_dedup: bool,
    pub fuzzy_threshold: f64,
    pub index_path: Option<PathBuf>,
    pub num_threads: usize,
    pub batch_size: usize,
    pub seed: u64,
}

impl Default for GlobalDedupConfig {
    fn default() -> Self {
        Self {
            num_permutations: 128,
            num_bands: 16,
            rows_per_band: 8,
            similarity_threshold: 0.8,
            shingle_size: 5,
            min_document_length: 50,
            max_document_length: 100000,
            use_exact_dedup: true,
            use_near_dedup: true,
            use_fuzzy_dedup: false,
            fuzzy_threshold: 0.7,
            index_path: None,
            num_threads: 4,
            batch_size: 10000,
            seed: 42,
        }
    }
}

impl GlobalDedupConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.num_permutations == 0 {
            return Err("num_permutations must be positive".to_string());
        }
        if self.num_bands == 0 {
            return Err("num_bands must be positive".to_string());
        }
        if self.rows_per_band == 0 {
            return Err("rows_per_band must be positive".to_string());
        }
        if self.num_bands * self.rows_per_band > self.num_permutations {
            return Err(format!(
                "num_bands * rows_per_band ({}) exceeds num_permutations ({})",
                self.num_bands * self.rows_per_band,
                self.num_permutations
            ));
        }
        if self.similarity_threshold <= 0.0 || self.similarity_threshold > 1.0 {
            return Err("similarity_threshold must be in (0, 1]".to_string());
        }
        Ok(())
    }

    pub fn estimated_threshold(&self) -> f64 {
        (1.0 / self.num_bands as f64).powf(1.0 / self.rows_per_band as f64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinHashSignature {
    pub doc_id: String,
    pub dataset_name: String,
    pub hash_values: Vec<u64>,
    pub doc_length: usize,
    pub exact_hash: u64,
}

impl MinHashSignature {
    pub fn compute(
        doc_id: &str,
        dataset_name: &str,
        text: &str,
        config: &GlobalDedupConfig,
    ) -> Option<Self> {
        let text = text.trim();
        if text.len() < config.min_document_length || text.len() > config.max_document_length {
            return None;
        }

        let shingles = generate_shingles(text, config.shingle_size);
        if shingles.is_empty() {
            return None;
        }

        let exact_hash = compute_exact_hash(text);

        let hash_values = compute_minhash(&shingles, config.num_permutations, config.seed);

        Some(Self {
            doc_id: doc_id.to_string(),
            dataset_name: dataset_name.to_string(),
            hash_values,
            doc_length: text.len(),
            exact_hash,
        })
    }

    pub fn band_hashes(&self, config: &GlobalDedupConfig) -> Vec<u64> {
        let mut band_hashes = Vec::with_capacity(config.num_bands);

        for band_idx in 0..config.num_bands {
            let start = band_idx * config.rows_per_band;
            let end = start + config.rows_per_band;

            let mut hasher = DefaultHasher::new();
            for &val in &self.hash_values[start..end.min(self.hash_values.len())] {
                val.hash(&mut hasher);
            }
            band_idx.hash(&mut hasher);
            band_hashes.push(hasher.finish());
        }

        band_hashes
    }

    pub fn estimate_jaccard(&self, other: &MinHashSignature) -> f64 {
        let len = self.hash_values.len().min(other.hash_values.len());
        if len == 0 {
            return 0.0;
        }

        let matches = self.hash_values[..len]
            .iter()
            .zip(&other.hash_values[..len])
            .filter(|(a, b)| a == b)
            .count();

        matches as f64 / len as f64
    }
}

fn generate_shingles(text: &str, shingle_size: usize) -> Vec<u64> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < shingle_size {
        return Vec::new();
    }

    let mut shingles = Vec::with_capacity(chars.len() - shingle_size + 1);

    for window in chars.windows(shingle_size) {
        let mut hasher = DefaultHasher::new();
        for &c in window {
            c.hash(&mut hasher);
        }
        shingles.push(hasher.finish());
    }

    shingles
}

fn compute_minhash(shingles: &[u64], num_permutations: usize, seed: u64) -> Vec<u64> {
    let mut signatures = vec![u64::MAX; num_permutations];

    for (perm_idx, sig) in signatures.iter_mut().enumerate() {
        let mut hasher = DefaultHasher::new();
        perm_idx.hash(&mut hasher);
        seed.hash(&mut hasher);

        let a = hasher.finish().wrapping_add(1);
        let b = hasher.finish().wrapping_add(2);

        for &shingle in shingles {
            let hash = a.wrapping_mul(shingle).wrapping_add(b);
            if hash < *sig {
                *sig = hash;
            }
        }
    }

    signatures
}

fn compute_exact_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSHIndex {
    config: GlobalDedupConfig,
    bands: Vec<HashMap<u64, Vec<usize>>>,
    signatures: Vec<MinHashSignature>,
    exact_index: HashMap<u64, Vec<usize>>,
    total_documents: usize,
    total_datasets: HashSet<String>,
}

impl LSHIndex {
    pub fn new(config: GlobalDedupConfig) -> Result<Self, String> {
        config.validate()?;

        Ok(Self {
            bands: vec![HashMap::new(); config.num_bands],
            signatures: Vec::new(),
            exact_index: HashMap::new(),
            total_documents: 0,
            total_datasets: HashSet::new(),
            config,
        })
    }

    pub fn insert(&mut self, signature: MinHashSignature) -> usize {
        let idx = self.signatures.len();

        if self.config.use_exact_dedup {
            self.exact_index
                .entry(signature.exact_hash)
                .or_default()
                .push(idx);
        }

        if self.config.use_near_dedup {
            let band_hashes = signature.band_hashes(&self.config);
            for (band_idx, &band_hash) in band_hashes.iter().enumerate() {
                self.bands[band_idx]
                    .entry(band_hash)
                    .or_default()
                    .push(idx);
            }
        }

        self.total_datasets.insert(signature.dataset_name.clone());
        self.signatures.push(signature);
        self.total_documents += 1;

        idx
    }

    pub fn insert_batch(&mut self, signatures: Vec<MinHashSignature>) -> Vec<usize> {
        let mut indices = Vec::with_capacity(signatures.len());
        for sig in signatures {
            indices.push(self.insert(sig));
        }
        indices
    }

    pub fn query_exact(&self, signature: &MinHashSignature) -> Vec<usize> {
        self.exact_index
            .get(&signature.exact_hash)
            .cloned()
            .unwrap_or_default()
    }

    pub fn query_near_duplicates(&self, signature: &MinHashSignature) -> Vec<(usize, f64)> {
        let band_hashes = signature.band_hashes(&self.config);
        let mut candidates: HashSet<usize> = HashSet::new();

        for (band_idx, &band_hash) in band_hashes.iter().enumerate() {
            if let Some(bucket) = self.bands[band_idx].get(&band_hash) {
                for &idx in bucket {
                    candidates.insert(idx);
                }
            }
        }

        let mut results = Vec::new();
        for &idx in &candidates {
            if idx < self.signatures.len() {
                let jaccard = signature.estimate_jaccard(&self.signatures[idx]);
                if jaccard >= self.config.similarity_threshold {
                    results.push((idx, jaccard));
                }
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    pub fn find_duplicates(&self, signature: &MinHashSignature) -> Vec<DuplicateInfo> {
        let mut duplicates = Vec::new();

        if self.config.use_exact_dedup {
            for &idx in &self.query_exact(signature) {
                if idx < self.signatures.len() {
                    let other = &self.signatures[idx];
                    duplicates.push(DuplicateInfo {
                        doc_id: other.doc_id.clone(),
                        dataset_name: other.dataset_name.clone(),
                        similarity: 1.0,
                        dup_type: DuplicateType::Exact,
                    });
                }
            }
        }

        if self.config.use_near_dedup {
            for (idx, similarity) in self.query_near_duplicates(signature) {
                if idx < self.signatures.len() {
                    let other = &self.signatures[idx];
                    let already_found = duplicates.iter().any(|d| d.doc_id == other.doc_id);
                    if !already_found {
                        duplicates.push(DuplicateInfo {
                            doc_id: other.doc_id.clone(),
                            dataset_name: other.dataset_name.clone(),
                            similarity,
                            dup_type: DuplicateType::NearDuplicate,
                        });
                    }
                }
            }
        }

        duplicates
    }

    pub fn deduplicate_batch(
        &mut self,
        signatures: Vec<MinHashSignature>,
    ) -> (Vec<MinHashSignature>, Vec<DuplicateInfo>) {
        let mut kept = Vec::new();
        let mut removed = Vec::new();

        for sig in signatures {
            let duplicates = self.find_duplicates(&sig);

            let is_duplicate = duplicates.iter().any(|d| {
                d.similarity >= self.config.similarity_threshold
            });

            if is_duplicate {
                removed.extend(duplicates);
            } else {
                self.insert(sig.clone());
                kept.push(sig);
            }
        }

        (kept, removed)
    }

    pub fn stats(&self) -> LSHStats {
        let mut per_dataset = HashMap::new();
        for sig in &self.signatures {
            *per_dataset.entry(sig.dataset_name.clone()).or_insert(0) += 1;
        }

        let total_bucket_entries: usize = self.bands.iter()
            .map(|band| band.values().map(|v| v.len()).sum::<usize>())
            .sum();

        let avg_bucket_size = if self.bands.iter().any(|b| !b.is_empty()) {
            let total_buckets: usize = self.bands.iter()
                .map(|b| b.len())
                .sum();
            if total_buckets > 0 {
                total_bucket_entries as f64 / total_buckets as f64
            } else {
                0.0
            }
        } else {
            0.0
        };

        LSHStats {
            total_documents: self.total_documents,
            total_datasets: self.total_datasets.len(),
            dataset_names: self.total_datasets.iter().cloned().collect(),
            per_dataset_documents: per_dataset,
            num_bands: self.config.num_bands,
            estimated_threshold: self.config.estimated_threshold(),
            total_bucket_entries,
            avg_bucket_size,
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), String> {
        let data = serde_json::to_string(&SerializableIndex {
            config: self.config.clone(),
            signatures: self.signatures.clone(),
        }).map_err(|e| format!("Serialization failed: {}", e))?;

        fs::write(path, data)
            .map_err(|e| format!("Failed to save index: {}", e))?;

        Ok(())
    }

    pub fn load(path: &PathBuf) -> Result<Self, String> {
        let data = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read index: {}", e))?;

        let saved: SerializableIndex = serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse index: {}", e))?;

        let mut index = Self::new(saved.config)?;
        for sig in saved.signatures {
            index.insert(sig);
        }

        Ok(index)
    }

    pub fn signatures(&self) -> &[MinHashSignature] {
        &self.signatures
    }

    pub fn len(&self) -> usize {
        self.total_documents
    }

    pub fn is_empty(&self) -> bool {
        self.total_documents == 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializableIndex {
    config: GlobalDedupConfig,
    signatures: Vec<MinHashSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateInfo {
    pub doc_id: String,
    pub dataset_name: String,
    pub similarity: f64,
    pub dup_type: DuplicateType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateType {
    Exact,
    NearDuplicate,
    Fuzzy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSHStats {
    pub total_documents: usize,
    pub total_datasets: usize,
    pub dataset_names: Vec<String>,
    pub per_dataset_documents: HashMap<String, usize>,
    pub num_bands: usize,
    pub estimated_threshold: f64,
    pub total_bucket_entries: usize,
    pub avg_bucket_size: f64,
}

pub struct GlobalDeduper {
    config: GlobalDedupConfig,
    index: LSHIndex,
    total_processed: usize,
    total_duplicates_found: usize,
    per_dataset_stats: HashMap<String, DatasetDedupStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetDedupStats {
    pub dataset_name: String,
    pub total_documents: usize,
    pub duplicates_found: usize,
    pub exact_duplicates: usize,
    pub near_duplicates: usize,
    pub fuzzy_duplicates: usize,
    pub dedup_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalDedupReport {
    pub total_documents_processed: usize,
    pub total_duplicates_found: usize,
    pub overall_dedup_ratio: f64,
    pub per_dataset: Vec<DatasetDedupStats>,
    pub cross_dataset_duplicates: usize,
    pub index_stats: LSHStats,
    pub config_summary: DedupConfigSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupConfigSummary {
    pub num_permutations: usize,
    pub num_bands: usize,
    pub rows_per_band: usize,
    pub similarity_threshold: f64,
    pub estimated_threshold: f64,
    pub shingle_size: usize,
}

impl GlobalDeduper {
    pub fn new(config: GlobalDedupConfig) -> Result<Self, String> {
        let index = LSHIndex::new(config.clone())?;

        Ok(Self {
            config,
            index,
            total_processed: 0,
            total_duplicates_found: 0,
            per_dataset_stats: HashMap::new(),
        })
    }

    pub fn process_document(
        &mut self,
        doc_id: &str,
        dataset_name: &str,
        text: &str,
    ) -> Option<Vec<DuplicateInfo>> {
        self.total_processed += 1;

        let signature = MinHashSignature::compute(doc_id, dataset_name, text, &self.config)?;

        let duplicates = self.index.find_duplicates(&signature);

        if duplicates.is_empty() {
            self.index.insert(signature);
            let stats = self.per_dataset_stats
                .entry(dataset_name.to_string())
                .or_insert_with(|| DatasetDedupStats {
                    dataset_name: dataset_name.to_string(),
                    total_documents: 0,
                    duplicates_found: 0,
                    exact_duplicates: 0,
                    near_duplicates: 0,
                    fuzzy_duplicates: 0,
                    dedup_ratio: 0.0,
                });
            stats.total_documents += 1;
            None
        } else {
            self.total_duplicates_found += 1;
            let stats = self.per_dataset_stats
                .entry(dataset_name.to_string())
                .or_insert_with(|| DatasetDedupStats {
                    dataset_name: dataset_name.to_string(),
                    total_documents: 0,
                    duplicates_found: 0,
                    exact_duplicates: 0,
                    near_duplicates: 0,
                    fuzzy_duplicates: 0,
                    dedup_ratio: 0.0,
                });
            stats.total_documents += 1;
            stats.duplicates_found += 1;

            for dup in &duplicates {
                match dup.dup_type {
                    DuplicateType::Exact => stats.exact_duplicates += 1,
                    DuplicateType::NearDuplicate => stats.near_duplicates += 1,
                    DuplicateType::Fuzzy => stats.fuzzy_duplicates += 1,
                }
            }

            Some(duplicates)
        }
    }

    pub fn process_batch(
        &mut self,
        documents: Vec<(String, String, String)>,
    ) -> Vec<(String, Vec<DuplicateInfo>)> {
        let mut results = Vec::new();

        for (doc_id, dataset_name, text) in documents {
            if let Some(dups) = self.process_document(&doc_id, &dataset_name, &text) {
                results.push((doc_id, dups));
            }
        }

        results
    }

    pub fn generate_report(&self) -> GlobalDedupReport {
        let mut per_dataset: Vec<DatasetDedupStats> = self.per_dataset_stats.values().cloned().collect();
        per_dataset.sort_by(|a, b| b.duplicates_found.cmp(&a.duplicates_found));

        for stats in &mut per_dataset {
            if stats.total_documents > 0 {
                stats.dedup_ratio = stats.duplicates_found as f64 / stats.total_documents as f64;
            }
        }

        let overall_ratio = if self.total_processed > 0 {
            self.total_duplicates_found as f64 / self.total_processed as f64
        } else {
            0.0
        };

        let cross_dataset = self.index.stats().total_datasets;

        GlobalDedupReport {
            total_documents_processed: self.total_processed,
            total_duplicates_found: self.total_duplicates_found,
            overall_dedup_ratio: overall_ratio,
            per_dataset,
            cross_dataset_duplicates: cross_dataset,
            index_stats: self.index.stats(),
            config_summary: DedupConfigSummary {
                num_permutations: self.config.num_permutations,
                num_bands: self.config.num_bands,
                rows_per_band: self.config.rows_per_band,
                similarity_threshold: self.config.similarity_threshold,
                estimated_threshold: self.config.estimated_threshold(),
                shingle_size: self.config.shingle_size,
            },
        }
    }

    pub fn index(&self) -> &LSHIndex {
        &self.index
    }

    pub fn stats(&self) -> LSHStats {
        self.index.stats()
    }

    pub fn save_index(&self, path: &PathBuf) -> Result<(), String> {
        self.index.save(path)
    }

    pub fn load_index(path: &PathBuf, config: GlobalDedupConfig) -> Result<Self, String> {
        let index = LSHIndex::load(path)?;

        Ok(Self {
            config,
            index,
            total_processed: 0,
            total_duplicates_found: 0,
            per_dataset_stats: HashMap::new(),
        })
    }
}

pub fn compute_text_similarity(text1: &str, text2: &str, shingle_size: usize) -> f64 {
    let shingles1: HashSet<u64> = generate_shingles(text1, shingle_size).into_iter().collect();
    let shingles2: HashSet<u64> = generate_shingles(text2, shingle_size).into_iter().collect();

    if shingles1.is_empty() && shingles2.is_empty() {
        return 1.0;
    }
    if shingles1.is_empty() || shingles2.is_empty() {
        return 0.0;
    }

    let intersection = shingles1.intersection(&shingles2).count();
    let union = shingles1.union(&shingles2).count();

    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn test_config() -> GlobalDedupConfig {
        GlobalDedupConfig {
            num_permutations: 64,
            num_bands: 8,
            rows_per_band: 8,
            similarity_threshold: 0.8,
            shingle_size: 3,
            min_document_length: 10,
            max_document_length: 10000,
            use_exact_dedup: true,
            use_near_dedup: true,
            use_fuzzy_dedup: false,
            fuzzy_threshold: 0.7,
            index_path: None,
            num_threads: 1,
            batch_size: 100,
            seed: 42,
        }
    }

    #[test]
    fn test_config_validation() {
        let config = test_config();
        assert!(config.validate().is_ok());

        let bad = GlobalDedupConfig {
            num_permutations: 0,
            ..Default::default()
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn test_shingle_generation() {
        let shingles = generate_shingles("hello world", 3);
        assert!(!shingles.is_empty());
    }

    #[test]
    fn test_shingle_empty() {
        let shingles = generate_shingles("hi", 5);
        assert!(shingles.is_empty());
    }

    #[test]
    fn test_minhash_signature() {
        let config = test_config();
        let sig = MinHashSignature::compute("doc1", "test", "This is a test document for minhash computation", &config);
        assert!(sig.is_some());

        let sig = sig.unwrap();
        assert_eq!(sig.hash_values.len(), config.num_permutations);
        assert_eq!(sig.doc_id, "doc1");
    }

    #[test]
    fn test_minhash_signature_too_short() {
        let config = test_config();
        let sig = MinHashSignature::compute("doc1", "test", "short", &config);
        assert!(sig.is_none());
    }

    #[test]
    fn test_band_hashes() {
        let config = test_config();
        let sig = MinHashSignature::compute(
            "doc1", "test",
            "This is a test document for band hash computation testing",
            &config,
        ).unwrap();

        let band_hashes = sig.band_hashes(&config);
        assert_eq!(band_hashes.len(), config.num_bands);
    }

    #[test]
    fn test_jaccard_estimation() {
        let config = test_config();
        let text = "This is a test document for jaccard estimation computation";
        let sig1 = MinHashSignature::compute("doc1", "test", text, &config).unwrap();
        let sig2 = MinHashSignature::compute("doc2", "test", text, &config).unwrap();

        let jaccard = sig1.estimate_jaccard(&sig2);
        assert!(jaccard > 0.9);
    }

    #[test]
    fn test_lsh_index_insert() {
        let config = test_config();
        let mut index = LSHIndex::new(config.clone()).unwrap();

        let sig = MinHashSignature::compute(
            "doc1", "test",
            "This is a test document for LSH index insertion",
            &config,
        ).unwrap();

        let idx = index.insert(sig);
        assert_eq!(idx, 0);
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_lsh_exact_dedup() {
        let config = test_config();
        let mut index = LSHIndex::new(config.clone()).unwrap();

        let text = "This is a unique test document for exact dedup testing";
        let sig1 = MinHashSignature::compute("doc1", "test", text, &config).unwrap();
        let sig2 = MinHashSignature::compute("doc2", "test", text, &config).unwrap();

        index.insert(sig1.clone());

        let dups = index.find_duplicates(&sig2);
        assert!(!dups.is_empty());
        assert!(dups.iter().any(|d| d.dup_type == DuplicateType::Exact));
    }

    #[test]
    fn test_lsh_near_dedup() {
        let config = test_config();
        let mut index = LSHIndex::new(config.clone()).unwrap();

        let text1 = "This is a test document for near dedup testing with some content";
        let text2 = "This is a test document for near dedup testing with some content and more";

        let sig1 = MinHashSignature::compute("doc1", "test", text1, &config).unwrap();
        index.insert(sig1);

        let sig2 = MinHashSignature::compute("doc2", "test", text2, &config).unwrap();
        let dups = index.find_duplicates(&sig2);

        assert!(!dups.is_empty());
    }

    #[test]
    fn test_lsh_different_docs() {
        let config = test_config();
        let mut index = LSHIndex::new(config.clone()).unwrap();

        let sig1 = MinHashSignature::compute(
            "doc1", "test",
            "This is a completely different document about machine learning and AI",
            &config,
        ).unwrap();
        index.insert(sig1);

        let sig2 = MinHashSignature::compute(
            "doc2", "test",
            "The weather today is sunny with a chance of rain in the afternoon",
            &config,
        ).unwrap();
        let dups = index.find_duplicates(&sig2);

        assert!(dups.is_empty() || dups.iter().all(|d| d.similarity < 0.8));
    }

    #[test]
    fn test_global_deduper() {
        let config = test_config();
        let mut deduper = GlobalDeduper::new(config).unwrap();

        let result1 = deduper.process_document(
            "doc1", "dataset_a",
            "This is the first unique document for global dedup testing",
        );
        assert!(result1.is_none());

        let result2 = deduper.process_document(
            "doc2", "dataset_a",
            "This is the first unique document for global dedup testing",
        );
        assert!(result2.is_some());

        let result3 = deduper.process_document(
            "doc3", "dataset_b",
            "This is a completely different document about science",
        );
        assert!(result3.is_none());
    }

    #[test]
    fn test_global_deduper_cross_dataset() {
        let config = test_config();
        let mut deduper = GlobalDeduper::new(config).unwrap();

        deduper.process_document(
            "doc1", "dataset_a",
            "Cross dataset dedup test document with unique content here",
        );

        let result = deduper.process_document(
            "doc2", "dataset_b",
            "Cross dataset dedup test document with unique content here",
        );

        assert!(result.is_some());
    }

    #[test]
    fn test_dedup_report() {
        let config = test_config();
        let mut deduper = GlobalDeduper::new(config).unwrap();

        deduper.process_document("doc1", "ds_a", "Unique document one for report testing");
        deduper.process_document("doc2", "ds_a", "Unique document one for report testing");
        deduper.process_document("doc3", "ds_b", "Another unique document for report testing");

        let report = deduper.generate_report();
        assert_eq!(report.total_documents_processed, 3);
        assert_eq!(report.total_duplicates_found, 1);
        assert!(report.per_dataset.len() >= 1);
    }

    #[test]
    fn test_text_similarity() {
        let sim = compute_text_similarity(
            "hello world this is a test",
            "hello world this is a test",
            3,
        );
        assert!((sim - 1.0).abs() < 0.01);

        let sim = compute_text_similarity(
            "hello world this is a test",
            "completely different content here",
            3,
        );
        assert!(sim < 0.5);
    }

    #[test]
    fn test_lsh_stats() {
        let config = test_config();
        let mut index = LSHIndex::new(config.clone()).unwrap();

        for i in 0..10 {
            let sig = MinHashSignature::compute(
                &format!("doc{}", i),
                "test",
                &format!("This is test document number {} for LSH stats testing", i),
                &config,
            ).unwrap();
            index.insert(sig);
        }

        let stats = index.stats();
        assert_eq!(stats.total_documents, 10);
        assert_eq!(stats.total_datasets, 1);
    }

    #[test]
    fn test_estimated_threshold() {
        let config = GlobalDedupConfig {
            num_bands: 16,
            rows_per_band: 8,
            ..Default::default()
        };
        let threshold = config.estimated_threshold();
        assert!(threshold > 0.0 && threshold < 1.0);
    }
}
