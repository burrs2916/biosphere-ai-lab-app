use serde::{Deserialize, Serialize};
use rand::SeedableRng;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackingConfig {
    pub max_sequence_length: usize,
    pub pad_token_id: i64,
    pub eos_token_id: Option<i64>,
    pub strategy: PackingStrategy,
    pub shuffle_before_pack: bool,
    pub seed: Option<u64>,
    pub add_position_ids: bool,
    pub cross_document_attention_mask: bool,
}

impl Default for PackingConfig {
    fn default() -> Self {
        Self {
            max_sequence_length: 2048,
            pad_token_id: 0,
            eos_token_id: None,
            strategy: PackingStrategy::BinPacking,
            shuffle_before_pack: true,
            seed: None,
            add_position_ids: true,
            cross_document_attention_mask: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackingStrategy {
    BinPacking,
    Greedy,
    BestFit,
    FirstFit,
    NoPack,
}

impl std::fmt::Display for PackingStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BinPacking => write!(f, "bin_packing"),
            Self::Greedy => write!(f, "greedy"),
            Self::BestFit => write!(f, "best_fit"),
            Self::FirstFit => write!(f, "first_fit"),
            Self::NoPack => write!(f, "no_pack"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackedSequence {
    pub token_ids: Vec<i64>,
    pub attention_mask: Vec<i64>,
    pub position_ids: Vec<i64>,
    pub sequence_boundaries: Vec<usize>,
    pub num_sequences: usize,
    pub original_indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackingReport {
    pub total_sequences: usize,
    pub packed_sequences: usize,
    pub total_tokens: usize,
    pub packed_tokens: usize,
    pub padding_tokens: usize,
    pub packing_efficiency: f64,
    pub avg_sequences_per_pack: f64,
    pub token_savings_pct: f64,
    pub strategy: PackingStrategy,
    pub max_sequence_length: usize,
}

pub struct SequencePacker;

impl SequencePacker {
    pub fn pack(
        sequences: &[Vec<i64>],
        config: &PackingConfig,
    ) -> Result<(Vec<PackedSequence>, PackingReport), String> {
        if sequences.is_empty() {
            return Err("No sequences to pack".to_string());
        }

        let mut seqs: Vec<(usize, Vec<i64>)> = sequences.iter()
            .enumerate()
            .map(|(i, s)| (i, s.clone()))
            .collect();

        if config.shuffle_before_pack {
            let mut rng: Box<dyn rand::RngCore> = if let Some(s) = config.seed {
                Box::new(rand::rngs::StdRng::seed_from_u64(s))
            } else {
                Box::new(rand::rngs::StdRng::from_entropy())
            };
            for i in (1..seqs.len()).rev() {
                let j = (rng.next_u64() as usize) % (i + 1);
                seqs.swap(i, j);
            }
        }

        let max_len = config.max_sequence_length;
        let eos = config.eos_token_id;

        let packed = match config.strategy {
            PackingStrategy::NoPack => {
                Self::no_pack(&seqs, max_len, eos)
            }
            PackingStrategy::FirstFit => {
                Self::first_fit_pack(&seqs, max_len, eos)
            }
            PackingStrategy::BestFit => {
                Self::best_fit_pack(&seqs, max_len, eos)
            }
            PackingStrategy::Greedy => {
                Self::greedy_pack(&seqs, max_len, eos)
            }
            PackingStrategy::BinPacking => {
                Self::bin_packing(&seqs, max_len, eos)
            }
        };

        let total_tokens: usize = sequences.iter().map(|s| s.len()).sum();
        let packed_tokens: usize = packed.iter().map(|p| p.token_ids.len()).sum();
        let padding_tokens = packed_tokens - total_tokens;
        let packing_efficiency = total_tokens as f64 / packed_tokens.max(1) as f64;
        let token_savings = if total_tokens > 0 {
            (1.0 - packed_tokens as f64 / (total_tokens as f64 * max_len as f64 / sequences.iter().map(|s| s.len()).max().unwrap_or(1) as f64)) * 100.0
        } else {
            0.0
        };

        let report = PackingReport {
            total_sequences: sequences.len(),
            packed_sequences: packed.len(),
            total_tokens,
            packed_tokens,
            padding_tokens,
            packing_efficiency,
            avg_sequences_per_pack: sequences.len() as f64 / packed.len().max(1) as f64,
            token_savings_pct: token_savings.max(0.0),
            strategy: config.strategy,
            max_sequence_length: max_len,
        };

        Ok((packed, report))
    }

    fn no_pack(
        sequences: &[(usize, Vec<i64>)],
        max_len: usize,
        eos: Option<i64>,
    ) -> Vec<PackedSequence> {
        sequences.iter().map(|(idx, seq)| {
            let mut token_ids = seq.clone();
            if let Some(e) = eos {
                token_ids.push(e);
            }
            let orig_len = token_ids.len();
            if token_ids.len() < max_len {
                token_ids.resize(max_len, 0);
            } else {
                token_ids.truncate(max_len);
            }

            let attention_mask: Vec<i64> = (0..max_len)
                .map(|i| if i < orig_len.min(max_len) { 1 } else { 0 })
                .collect();

            let position_ids: Vec<i64> = (0..max_len).map(|i| i as i64).collect();

            PackedSequence {
                token_ids,
                attention_mask,
                position_ids,
                sequence_boundaries: vec![orig_len.min(max_len)],
                num_sequences: 1,
                original_indices: vec![*idx],
            }
        }).collect()
    }

    fn first_fit_pack(
        sequences: &[(usize, Vec<i64>)],
        max_len: usize,
        eos: Option<i64>,
    ) -> Vec<PackedSequence> {
        let mut bins: Vec<(Vec<i64>, Vec<usize>, Vec<usize>)> = Vec::new();

        for (idx, seq) in sequences {
            let mut seq_len = seq.len();
            if eos.is_some() {
                seq_len += 1;
            }

            let mut placed = false;
            for (tokens, boundaries, indices) in &mut bins {
                if tokens.len() + seq_len <= max_len {
                    tokens.extend(seq.iter().cloned());
                    if let Some(e) = eos {
                        tokens.push(e);
                    }
                    boundaries.push(tokens.len());
                    indices.push(*idx);
                    placed = true;
                    break;
                }
            }

            if !placed {
                let mut tokens = seq.clone();
                if let Some(e) = eos {
                    tokens.push(e);
                }
                let boundaries = vec![tokens.len()];
                let indices = vec![*idx];
                bins.push((tokens, boundaries, indices));
            }
        }

        bins.into_iter().map(|(tokens, boundaries, indices)| {
            let orig_len = tokens.len();
            let mut padded = tokens;
            padded.resize(max_len, 0);

            let attention_mask: Vec<i64> = (0..max_len)
                .map(|i| if i < orig_len { 1 } else { 0 })
                .collect();

            let position_ids: Vec<i64> = (0..max_len).map(|i| i as i64).collect();

            PackedSequence {
                token_ids: padded,
                attention_mask,
                position_ids,
                sequence_boundaries: boundaries,
                num_sequences: indices.len(),
                original_indices: indices,
            }
        }).collect()
    }

    fn best_fit_pack(
        sequences: &[(usize, Vec<i64>)],
        max_len: usize,
        eos: Option<i64>,
    ) -> Vec<PackedSequence> {
        let mut sorted: Vec<(usize, Vec<i64>)> = sequences.to_vec();
        sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        let mut bins: Vec<(Vec<i64>, Vec<usize>, Vec<usize>)> = Vec::new();

        for (idx, seq) in &sorted {
            let mut seq_len = seq.len();
            if eos.is_some() {
                seq_len += 1;
            }

            let mut best_bin: Option<usize> = None;
            let mut best_remaining = usize::MAX;

            for (bin_idx, (tokens, _, _)) in bins.iter().enumerate() {
                let remaining = max_len - tokens.len();
                if remaining >= seq_len && remaining < best_remaining {
                    best_remaining = remaining;
                    best_bin = Some(bin_idx);
                }
            }

            if let Some(bin_idx) = best_bin {
                let (tokens, boundaries, indices) = &mut bins[bin_idx];
                tokens.extend(seq.iter().cloned());
                if let Some(e) = eos {
                    tokens.push(e);
                }
                boundaries.push(tokens.len());
                indices.push(*idx);
            } else {
                let mut tokens = seq.clone();
                if let Some(e) = eos {
                    tokens.push(e);
                }
                let tok_len = tokens.len();
                bins.push((tokens, vec![tok_len], vec![*idx]));
            }
        }

        bins.into_iter().map(|(tokens, boundaries, indices)| {
            let orig_len = tokens.len();
            let mut padded = tokens;
            padded.resize(max_len, 0);

            let attention_mask: Vec<i64> = (0..max_len)
                .map(|i| if i < orig_len { 1 } else { 0 })
                .collect();

            let position_ids: Vec<i64> = (0..max_len).map(|i| i as i64).collect();

            PackedSequence {
                token_ids: padded,
                attention_mask,
                position_ids,
                sequence_boundaries: boundaries,
                num_sequences: indices.len(),
                original_indices: indices,
            }
        }).collect()
    }

    fn greedy_pack(
        sequences: &[(usize, Vec<i64>)],
        max_len: usize,
        eos: Option<i64>,
    ) -> Vec<PackedSequence> {
        let mut sorted: Vec<(usize, Vec<i64>)> = sequences.to_vec();
        sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        let mut bins: Vec<(Vec<i64>, Vec<usize>, Vec<usize>)> = Vec::new();

        for (idx, seq) in &sorted {
            let mut seq_len = seq.len();
            if eos.is_some() {
                seq_len += 1;
            }

            let mut placed = false;
            for (tokens, boundaries, indices) in &mut bins {
                if tokens.len() + seq_len <= max_len {
                    tokens.extend(seq.iter().cloned());
                    if let Some(e) = eos {
                        tokens.push(e);
                    }
                    boundaries.push(tokens.len());
                    indices.push(*idx);
                    placed = true;
                    break;
                }
            }

            if !placed {
                let mut tokens = seq.clone();
                if let Some(e) = eos {
                    tokens.push(e);
                }
                let tok_len = tokens.len();
                bins.push((tokens, vec![tok_len], vec![*idx]));
            }
        }

        bins.into_iter().map(|(tokens, boundaries, indices)| {
            let orig_len = tokens.len();
            let mut padded = tokens;
            padded.resize(max_len, 0);

            let attention_mask: Vec<i64> = (0..max_len)
                .map(|i| if i < orig_len { 1 } else { 0 })
                .collect();

            let position_ids: Vec<i64> = (0..max_len).map(|i| i as i64).collect();

            PackedSequence {
                token_ids: padded,
                attention_mask,
                position_ids,
                sequence_boundaries: boundaries,
                num_sequences: indices.len(),
                original_indices: indices,
            }
        }).collect()
    }

    fn bin_packing(
        sequences: &[(usize, Vec<i64>)],
        max_len: usize,
        eos: Option<i64>,
    ) -> Vec<PackedSequence> {
        let mut items: Vec<(usize, usize, Vec<i64>)> = sequences.iter()
            .map(|(idx, seq)| {
                let len = seq.len() + if eos.is_some() { 1 } else { 0 };
                (*idx, len, seq.clone())
            })
            .collect();

        items.sort_by(|a, b| b.1.cmp(&a.1));

        let mut bins: Vec<(Vec<i64>, Vec<usize>, Vec<usize>)> = Vec::new();

        for (idx, seq_len, seq) in &items {
            let mut placed = false;

            for (tokens, boundaries, indices) in &mut bins {
                if tokens.len() + seq_len <= max_len {
                    tokens.extend(seq.iter().cloned());
                    if let Some(e) = eos {
                        tokens.push(e);
                    }
                    boundaries.push(tokens.len());
                    indices.push(*idx);
                    placed = true;
                    break;
                }
            }

            if !placed {
                let mut tokens = seq.clone();
                if let Some(e) = eos {
                    tokens.push(e);
                }
                let tok_len = tokens.len();
                bins.push((tokens, vec![tok_len], vec![*idx]));
            }
        }

        bins.into_iter().map(|(tokens, boundaries, indices)| {
            let orig_len = tokens.len();
            let mut padded = tokens;
            padded.resize(max_len, 0);

            let attention_mask: Vec<i64> = (0..max_len)
                .map(|i| if i < orig_len { 1 } else { 0 })
                .collect();

            let position_ids: Vec<i64> = (0..max_len).map(|i| i as i64).collect();

            PackedSequence {
                token_ids: padded,
                attention_mask,
                position_ids,
                sequence_boundaries: boundaries,
                num_sequences: indices.len(),
                original_indices: indices,
            }
        }).collect()
    }

    pub fn estimate_packing_efficiency(
        sequence_lengths: &[usize],
        max_sequence_length: usize,
    ) -> f64 {
        if sequence_lengths.is_empty() {
            return 0.0;
        }

        let total_tokens: usize = sequence_lengths.iter().sum();
        let mut sorted = sequence_lengths.to_vec();
        sorted.sort_by(|a, b| b.cmp(a));

        let mut bins: Vec<usize> = Vec::new();
        for &len in &sorted {
            let mut placed = false;
            for bin in &mut bins {
                if *bin + len <= max_sequence_length {
                    *bin += len;
                    placed = true;
                    break;
                }
            }
            if !placed {
                bins.push(len);
            }
        }

        let total_capacity = bins.len() * max_sequence_length;
        if total_capacity == 0 {
            return 0.0;
        }
        total_tokens as f64 / total_capacity as f64
    }
}
