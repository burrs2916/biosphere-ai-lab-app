use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::data::tokenizer::BatchEncoding;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataCollatorConfig {
    pub pad_to_multiple_of: Option<usize>,
    pub return_tensors: String,
    pub label_pad_token_id: i64,
    pub mlm_probability: Option<f64>,
}

impl Default for DataCollatorConfig {
    fn default() -> Self {
        Self {
            pad_to_multiple_of: Some(8),
            return_tensors: "arrow".to_string(),
            label_pad_token_id: -100,
            mlm_probability: None,
        }
    }
}

pub struct DataCollator {
    config: DataCollatorConfig,
    pad_token_id: u32,
}

impl DataCollator {
    pub fn new(pad_token_id: u32, config: DataCollatorConfig) -> Self {
        Self { config, pad_token_id }
    }

    pub fn collate(&self, batch: &[BatchEncoding]) -> Result<BatchEncoding, String> {
        if batch.is_empty() {
            return Err("Cannot collate empty batch".to_string());
        }

        let mut all_input_ids = Vec::new();
        let mut all_attention_mask = Vec::new();
        let mut all_length = Vec::new();

        for encoding in batch {
            for (i, ids) in encoding.input_ids.iter().enumerate() {
                all_input_ids.push(ids.clone());
                all_length.push(ids.len());
                if i < encoding.attention_mask.len() {
                    all_attention_mask.push(encoding.attention_mask[i].clone());
                } else {
                    all_attention_mask.push(vec![1u8; ids.len()]);
                }
            }
        }

        let max_len = all_input_ids.iter().map(|ids| ids.len()).max().unwrap_or(0);
        let padded_len = match self.config.pad_to_multiple_of {
            Some(multiple) if multiple > 0 => {
                ((max_len + multiple - 1) / multiple) * multiple
            }
            _ => max_len,
        };

        for (ids, mask) in all_input_ids.iter_mut().zip(all_attention_mask.iter_mut()) {
            while ids.len() < padded_len {
                ids.push(self.pad_token_id);
                mask.push(0);
            }
        }

        let mut all_labels = all_input_ids.clone();
        for (labels, mask) in all_labels.iter_mut().zip(all_attention_mask.iter()) {
            for (label, &m) in labels.iter_mut().zip(mask.iter()) {
                if m == 0 {
                    *label = self.config.label_pad_token_id as u32;
                }
            }
        }

        Ok(BatchEncoding {
            input_ids: all_input_ids,
            attention_mask: all_attention_mask,
            token_type_ids: None,
            special_tokens_mask: None,
            length: all_length,
            overflow_to_sample_mapping: None,
            metadata: HashMap::new(),
        })
    }

    pub fn collate_with_labels(
        &self,
        features: &[CollatedFeature],
    ) -> Result<BatchEncoding, String> {
        if features.is_empty() {
            return Err("Cannot collate empty features".to_string());
        }

        let mut all_input_ids = Vec::new();
        let mut all_attention_mask = Vec::new();
        let mut all_labels = Vec::new();
        let mut all_length = Vec::new();

        for feature in features {
            all_input_ids.push(feature.input_ids.clone());
            all_attention_mask.push(feature.attention_mask.clone());
            all_labels.push(feature.labels.clone());
            all_length.push(feature.input_ids.len());
        }

        let max_len = all_input_ids.iter().map(|ids| ids.len()).max().unwrap_or(0);
        let padded_len = match self.config.pad_to_multiple_of {
            Some(multiple) if multiple > 0 => {
                ((max_len + multiple - 1) / multiple) * multiple
            }
            _ => max_len,
        };

        for (ids, mask) in all_input_ids.iter_mut().zip(all_attention_mask.iter_mut()) {
            while ids.len() < padded_len {
                ids.push(self.pad_token_id);
                mask.push(0);
            }
        }

        for labels in all_labels.iter_mut() {
            while labels.len() < padded_len {
                labels.push(self.config.label_pad_token_id as u32);
            }
        }

        Ok(BatchEncoding {
            input_ids: all_input_ids,
            attention_mask: all_attention_mask,
            token_type_ids: None,
            special_tokens_mask: None,
            length: all_length,
            overflow_to_sample_mapping: None,
            metadata: HashMap::new(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollatedFeature {
    pub input_ids: Vec<u32>,
    pub attention_mask: Vec<u8>,
    pub labels: Vec<u32>,
    pub token_type_ids: Option<Vec<u8>>,
}

pub struct DataCollatorForSeq2Seq {
    collator: DataCollator,
    model_type: Seq2SeqModelType,
    decoder_start_token_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Seq2SeqModelType {
    T5,
    Bart,
    Pegasus,
    Generic,
}

impl DataCollatorForSeq2Seq {
    pub fn new(
        pad_token_id: u32,
        config: DataCollatorConfig,
        model_type: Seq2SeqModelType,
        decoder_start_token_id: Option<u32>,
    ) -> Self {
        Self {
            collator: DataCollator::new(pad_token_id, config),
            model_type,
            decoder_start_token_id,
        }
    }

    pub fn collate_seq2seq(
        &self,
        features: &[Seq2SeqFeature],
    ) -> Result<Seq2SeqBatch, String> {
        if features.is_empty() {
            return Err("Cannot collate empty seq2seq features".to_string());
        }

        let mut input_ids = Vec::new();
        let mut attention_mask = Vec::new();
        let mut decoder_input_ids = Vec::new();
        let mut decoder_attention_mask = Vec::new();
        let mut labels = Vec::new();

        for feature in features {
            input_ids.push(feature.input_ids.clone());
            attention_mask.push(feature.attention_mask.clone());
            decoder_input_ids.push(feature.decoder_input_ids.clone());
            decoder_attention_mask.push(feature.decoder_attention_mask.clone());
            labels.push(feature.labels.clone());
        }

        let max_enc_len = input_ids.iter().map(|ids| ids.len()).max().unwrap_or(0);
        let max_dec_len = decoder_input_ids.iter().map(|ids| ids.len()).max().unwrap_or(0);

        let pad_id = self.collator.pad_token_id;
        let label_pad = self.collator.config.label_pad_token_id as u32;

        for (ids, mask) in input_ids.iter_mut().zip(attention_mask.iter_mut()) {
            while ids.len() < max_enc_len {
                ids.push(pad_id);
                mask.push(0);
            }
        }

        for (ids, mask) in decoder_input_ids.iter_mut().zip(decoder_attention_mask.iter_mut()) {
            while ids.len() < max_dec_len {
                ids.push(pad_id);
                mask.push(0);
            }
        }

        for lbl in labels.iter_mut() {
            while lbl.len() < max_dec_len {
                lbl.push(label_pad);
            }
        }

        Ok(Seq2SeqBatch {
            input_ids,
            attention_mask,
            decoder_input_ids,
            decoder_attention_mask,
            labels,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seq2SeqFeature {
    pub input_ids: Vec<u32>,
    pub attention_mask: Vec<u8>,
    pub decoder_input_ids: Vec<u32>,
    pub decoder_attention_mask: Vec<u8>,
    pub labels: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seq2SeqBatch {
    pub input_ids: Vec<Vec<u32>>,
    pub attention_mask: Vec<Vec<u8>>,
    pub decoder_input_ids: Vec<Vec<u32>>,
    pub decoder_attention_mask: Vec<Vec<u8>>,
    pub labels: Vec<Vec<u32>>,
}

pub struct DataCollatorForLanguageModeling {
    collator: DataCollator,
    mlm_probability: f64,
    mask_token_id: u32,
    vocab_size: usize,
}

impl DataCollatorForLanguageModeling {
    pub fn new(
        pad_token_id: u32,
        mask_token_id: u32,
        vocab_size: usize,
        config: DataCollatorConfig,
    ) -> Self {
        let mlm_prob = config.mlm_probability.unwrap_or(0.15);
        Self {
            collator: DataCollator::new(pad_token_id, config),
            mlm_probability: mlm_prob,
            mask_token_id,
            vocab_size,
        }
    }

    pub fn mask_tokens(
        &self,
        inputs: &[Vec<u32>],
        special_tokens_mask: &[Vec<u8>],
    ) -> (Vec<Vec<u32>>, Vec<Vec<u32>>) {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let mut masked_inputs = Vec::with_capacity(inputs.len());
        let mut labels = Vec::with_capacity(inputs.len());

        for (input, special_mask) in inputs.iter().zip(special_tokens_mask.iter()) {
            let mut masked = input.clone();
            let mut label = vec![self.collator.config.label_pad_token_id as u32; input.len()];

            let candidate_indices: Vec<usize> = input.iter().enumerate()
                .filter(|(i, _)| special_mask.get(*i).map_or(true, |&m| m == 0))
                .map(|(i, _)| i)
                .collect();

            let num_to_mask = (candidate_indices.len() as f64 * self.mlm_probability).ceil() as usize;
            let num_to_mask = num_to_mask.min(candidate_indices.len());

            let mut selected: Vec<usize> = candidate_indices.clone();
            use rand::seq::SliceRandom;
            selected.shuffle(&mut rng);
            selected.truncate(num_to_mask);

            for &idx in &selected {
                label[idx] = input[idx];
                let rand_val: f64 = rng.gen();
                if rand_val < 0.8 {
                    masked[idx] = self.mask_token_id;
                } else if rand_val < 0.9 {
                    masked[idx] = rng.gen_range(0u32..self.vocab_size as u32);
                }
            }

            masked_inputs.push(masked);
            labels.push(label);
        }

        (masked_inputs, labels)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollationReport {
    pub num_samples: usize,
    pub max_sequence_length: usize,
    pub padded_sequence_length: usize,
    pub padding_ratio: f64,
    pub total_tokens: usize,
    pub effective_tokens: usize,
    pub padding_efficiency: f64,
}

pub fn compute_collation_report(batch: &BatchEncoding) -> CollationReport {
    let num_samples = batch.num_samples();
    let max_len = batch.max_sequence_length();
    let total_tokens = num_samples * max_len;

    let effective_tokens: usize = batch.length.iter().sum();
    let padding_ratio = if total_tokens > 0 {
        1.0 - (effective_tokens as f64 / total_tokens as f64)
    } else {
        0.0
    };

    CollationReport {
        num_samples,
        max_sequence_length: max_len,
        padded_sequence_length: max_len,
        padding_ratio,
        total_tokens,
        effective_tokens,
        padding_efficiency: 1.0 - padding_ratio,
    }
}
