use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupReport {
    pub dataset_id: String,
    pub total_rows: usize,
    pub exact_duplicates: ExactDedupResult,
    pub near_duplicates: Option<NearDedupResult>,
    pub overall_duplicate_rate: f64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExactDedupResult {
    pub duplicate_groups: usize,
    pub duplicate_rows: usize,
    pub unique_rows: usize,
    pub duplicate_rate: f64,
    pub duplicate_indices: Vec<Vec<usize>>,
    pub largest_group_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearDedupResult {
    pub near_duplicate_pairs: usize,
    pub near_duplicate_clusters: usize,
    pub affected_rows: usize,
    pub similarity_threshold: f64,
    pub cluster_sizes: Vec<usize>,
    pub sample_pairs: Vec<NearDuplicatePair>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearDuplicatePair {
    pub row_a: usize,
    pub row_b: usize,
    pub similarity: f64,
    pub diff_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupConfig {
    pub exact_dedup: bool,
    pub near_dedup: bool,
    pub similarity_threshold: f64,
    pub num_hashes: usize,
    pub shingle_size: usize,
    pub columns_to_check: Option<Vec<String>>,
    pub ignore_case: bool,
    pub ignore_whitespace: bool,
}

impl Default for DedupConfig {
    fn default() -> Self {
        Self {
            exact_dedup: true,
            near_dedup: true,
            similarity_threshold: 0.8,
            num_hashes: 128,
            shingle_size: 3,
            columns_to_check: None,
            ignore_case: true,
            ignore_whitespace: true,
        }
    }
}

pub struct DedupAnalyzer;

impl DedupAnalyzer {
    pub fn analyze(
        dataset_id: &str,
        rows: &[Vec<String>],
        column_names: &[String],
        config: &DedupConfig,
    ) -> DedupReport {
        let total_rows = rows.len();

        let exact_result = Self::exact_dedup(rows, config);

        let near_result = if config.near_dedup {
            Some(Self::near_dedup(rows, column_names, config))
        } else {
            None
        };

        let overall_duplicate_rate = if let Some(ref near) = near_result {
            let total_dup = exact_result.duplicate_rows + near.affected_rows;
            (total_dup as f64 / total_rows as f64).min(1.0)
        } else {
            exact_result.duplicate_rate
        };

        let mut recommendations = Vec::new();

        if exact_result.duplicate_rate > 0.0 {
            recommendations.push(format!(
                "发现 {:.1}% 精确重复行（{} 行），建议去重后训练",
                exact_result.duplicate_rate * 100.0,
                exact_result.duplicate_rows
            ));
        }

        if exact_result.largest_group_size > 10 {
            recommendations.push(format!(
                "⚠️ 最大重复组包含 {} 行，可能存在数据采集问题",
                exact_result.largest_group_size
            ));
        }

        if let Some(ref near) = near_result {
            if near.near_duplicate_pairs > 0 {
                recommendations.push(format!(
                    "发现 {} 对近似重复（相似度 ≥ {:.0}%），建议检查是否为标注错误",
                    near.near_duplicate_pairs,
                    config.similarity_threshold * 100.0
                ));
            }
        }

        if overall_duplicate_rate < 0.01 {
            recommendations.push("✅ 数据重复率低，数据质量良好".to_string());
        }

        DedupReport {
            dataset_id: dataset_id.to_string(),
            total_rows,
            exact_duplicates: exact_result,
            near_duplicates: near_result,
            overall_duplicate_rate,
            recommendations,
        }
    }

    fn exact_dedup(rows: &[Vec<String>], config: &DedupConfig) -> ExactDedupResult {
        let mut seen: HashMap<String, Vec<usize>> = HashMap::new();

        for (idx, row) in rows.iter().enumerate() {
            let key = Self::normalize_row(row, config);
            seen.entry(key).or_default().push(idx);
        }

        let duplicate_groups: Vec<Vec<usize>> = seen.into_values()
            .filter(|indices| indices.len() > 1)
            .collect();

        let duplicate_rows: usize = duplicate_groups.iter()
            .map(|g| g.len() - 1)
            .sum();

        let unique_rows = rows.len() - duplicate_rows;
        let duplicate_rate = duplicate_rows as f64 / rows.len() as f64;
        let largest_group_size = duplicate_groups.iter()
            .map(|g| g.len())
            .max()
            .unwrap_or(0);

        ExactDedupResult {
            duplicate_groups: duplicate_groups.len(),
            duplicate_rows,
            unique_rows,
            duplicate_rate,
            duplicate_indices: duplicate_groups,
            largest_group_size,
        }
    }

    fn near_dedup(
        rows: &[Vec<String>],
        column_names: &[String],
        config: &DedupConfig,
    ) -> NearDedupResult {
        let signatures: Vec<u64> = rows.iter()
            .map(|row| Self::minhash_signature(row, config))
            .collect();

        let mut pairs = Vec::new();
        let mut checked: HashSet<(usize, usize)> = HashSet::new();

        let band_size = 4;
        let num_bands = config.num_hashes / band_size;

        for band in 0..num_bands {
            let mut buckets: HashMap<u64, Vec<usize>> = HashMap::new();
            let start = band * band_size;
            let end = start + band_size;

            for (idx, sig) in signatures.iter().enumerate() {
                let bucket_key = Self::band_hash(sig, start, end);
                buckets.entry(bucket_key).or_default().push(idx);
            }

            for indices in buckets.values() {
                if indices.len() < 2 || indices.len() > 100 {
                    continue;
                }
                for i in 0..indices.len() {
                    for j in (i + 1)..indices.len() {
                        let a = indices[i];
                        let b = indices[j];
                        let pair = if a < b { (a, b) } else { (b, a) };
                        if checked.contains(&pair) {
                            continue;
                        }
                        checked.insert(pair);

                        let similarity = Self::jaccard_similarity(&rows[a], &rows[b], config);
                        if similarity >= config.similarity_threshold {
                            let diff_fields: Vec<String> = rows[a].iter()
                                .zip(rows[b].iter())
                                .enumerate()
                                .filter(|(_, (va, vb))| va != vb)
                                .map(|(ci, _)| {
                                    if ci < column_names.len() {
                                        column_names[ci].clone()
                                    } else {
                                        format!("col_{}", ci)
                                    }
                                })
                                .collect();

                            pairs.push(NearDuplicatePair {
                                row_a: a,
                                row_b: b,
                                similarity,
                                diff_fields,
                            });
                        }
                    }
                }
            }
        }

        let mut affected_set: HashSet<usize> = HashSet::new();
        for pair in &pairs {
            affected_set.insert(pair.row_a);
            affected_set.insert(pair.row_b);
        }

        let clusters = Self::cluster_pairs(&pairs);
        let cluster_sizes: Vec<usize> = clusters.iter().map(|c| c.len()).collect();

        NearDedupResult {
            near_duplicate_pairs: pairs.len(),
            near_duplicate_clusters: clusters.len(),
            affected_rows: affected_set.len(),
            similarity_threshold: config.similarity_threshold,
            cluster_sizes,
            sample_pairs: pairs.into_iter().take(20).collect(),
        }
    }

    fn normalize_row(row: &[String], config: &DedupConfig) -> String {
        row.iter()
            .map(|s| {
                let mut s = s.clone();
                if config.ignore_case {
                    s = s.to_lowercase();
                }
                if config.ignore_whitespace {
                    s = s.trim().to_string();
                }
                s
            })
            .collect::<Vec<_>>()
            .join("|")
    }

    fn minhash_signature(row: &[String], config: &DedupConfig) -> u64 {
        let text = row.join(" ");
        let shingles: Vec<String> = text.chars()
            .collect::<Vec<char>>()
            .windows(config.shingle_size)
            .map(|w| w.iter().collect())
            .collect();

        if shingles.is_empty() {
            return 0;
        }

        let mut min_hash = u64::MAX;
        for shingle in &shingles {
            let h = Self::fnv_hash(shingle);
            min_hash = min_hash.min(h);
        }

        min_hash
    }

    fn fnv_hash(s: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        let prime: u64 = 0x100000001b3;
        for byte in s.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(prime);
        }
        hash
    }

    fn band_hash(signature: &u64, _start: usize, _end: usize) -> u64 {
        *signature
    }

    fn jaccard_similarity(a: &[String], b: &[String], config: &DedupConfig) -> f64 {
        let set_a: HashSet<String> = a.iter()
            .map(|s| {
                let mut s = s.clone();
                if config.ignore_case { s = s.to_lowercase(); }
                if config.ignore_whitespace { s = s.trim().to_string(); }
                s
            })
            .collect();

        let set_b: HashSet<String> = b.iter()
            .map(|s| {
                let mut s = s.clone();
                if config.ignore_case { s = s.to_lowercase(); }
                if config.ignore_whitespace { s = s.trim().to_string(); }
                s
            })
            .collect();

        let intersection = set_a.intersection(&set_b).count();
        let union = set_a.union(&set_b).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    fn cluster_pairs(pairs: &[NearDuplicatePair]) -> Vec<Vec<usize>> {
        let mut parent: HashMap<usize, usize> = HashMap::new();

        fn find(parent: &mut HashMap<usize, usize>, x: usize) -> usize {
            let mut current = x;
            let mut path = Vec::new();
            while let Some(&p) = parent.get(&current) {
                if p == current {
                    break;
                }
                path.push(current);
                current = p;
            }
            let root = current;
            for node in path {
                parent.insert(node, root);
            }
            root
        }

        fn union(parent: &mut HashMap<usize, usize>, a: usize, b: usize) {
            let ra = find(parent, a);
            let rb = find(parent, b);
            if ra != rb {
                parent.insert(ra, rb);
            }
        }

        for pair in pairs {
            union(&mut parent, pair.row_a, pair.row_b);
        }

        let mut clusters: HashMap<usize, Vec<usize>> = HashMap::new();
        let all_indices: HashSet<usize> = pairs.iter()
            .flat_map(|p| [p.row_a, p.row_b])
            .collect();

        for idx in all_indices {
            let root = find(&mut parent, idx);
            clusters.entry(root).or_default().push(idx);
        }

        clusters.into_values().collect()
    }

    pub fn compute_digest(rows: &[Vec<String>]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for row in rows {
            for val in row {
                val.hash(&mut hasher);
            }
        }
        format!("{:x}", hasher.finish())
    }
}
