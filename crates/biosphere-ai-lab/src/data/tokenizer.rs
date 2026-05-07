use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::data::arrow_table::ArrowTable;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizerConfig {
    pub tokenizer_path: String,
    pub max_length: usize,
    pub truncation: bool,
    pub padding: PaddingStrategy,
    pub add_special_tokens: bool,
    pub return_attention_mask: bool,
    pub return_token_type_ids: bool,
}

impl Default for TokenizerConfig {
    fn default() -> Self {
        Self {
            tokenizer_path: String::new(),
            max_length: 512,
            truncation: true,
            padding: PaddingStrategy::Longest,
            add_special_tokens: true,
            return_attention_mask: true,
            return_token_type_ids: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaddingStrategy {
    Longest,
    MaxLength,
    DoNotPad,
    Fixed(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEncoding {
    pub input_ids: Vec<Vec<u32>>,
    pub attention_mask: Vec<Vec<u8>>,
    pub token_type_ids: Option<Vec<Vec<u8>>>,
    pub special_tokens_mask: Option<Vec<Vec<u8>>>,
    pub length: Vec<usize>,
    pub overflow_to_sample_mapping: Option<Vec<usize>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl BatchEncoding {
    pub fn num_samples(&self) -> usize {
        self.input_ids.len()
    }

    pub fn max_sequence_length(&self) -> usize {
        self.input_ids.iter().map(|ids| ids.len()).max().unwrap_or(0)
    }

    pub fn to_arrow_table(&self, name: &str) -> Result<ArrowTable, String> {
        use arrow::datatypes::{DataType, Field, Schema};

        let mut fields: Vec<Field> = vec![
            Field::new("input_ids", DataType::List(std::sync::Arc::new(Field::new("item", DataType::UInt32, true))), false),
            Field::new("attention_mask", DataType::List(std::sync::Arc::new(Field::new("item", DataType::UInt8, true))), false),
        ];

        if self.token_type_ids.is_some() {
            fields.push(Field::new("token_type_ids", DataType::List(std::sync::Arc::new(Field::new("item", DataType::UInt8, true))), true));
        }

        let schema = std::sync::Arc::new(Schema::new(fields));
        let mut table = ArrowTable::new(name, schema.clone());
        let num_samples = self.num_samples();

        let mut input_ids_builder = arrow::array::ListBuilder::new(arrow::array::UInt32Builder::new());
        let mut attention_mask_builder = arrow::array::ListBuilder::new(arrow::array::UInt8Builder::new());
        let mut token_type_ids_builder: Option<arrow::array::ListBuilder<arrow::array::UInt8Builder>> =
            if self.token_type_ids.is_some() {
                Some(arrow::array::ListBuilder::new(arrow::array::UInt8Builder::new()))
            } else {
                None
            };

        for i in 0..num_samples {
            {
                let values = input_ids_builder.values();
                for &id in &self.input_ids[i] {
                    values.append_value(id);
                }
                input_ids_builder.append(true);
            }
            {
                let values = attention_mask_builder.values();
                for &mask in &self.attention_mask[i] {
                    values.append_value(mask);
                }
                attention_mask_builder.append(true);
            }
            if let Some(ref mut builder) = token_type_ids_builder {
                if let Some(ref type_ids) = self.token_type_ids {
                    let values = builder.values();
                    for &tid in &type_ids[i] {
                        values.append_value(tid);
                    }
                    builder.append(true);
                }
            }
        }

        let mut arrays: Vec<std::sync::Arc<dyn arrow::array::Array>> = vec![
            std::sync::Arc::new(input_ids_builder.finish()),
            std::sync::Arc::new(attention_mask_builder.finish()),
        ];

        if let Some(mut builder) = token_type_ids_builder {
            arrays.push(std::sync::Arc::new(builder.finish()));
        }

        let batch = arrow::record_batch::RecordBatch::try_new(schema, arrays)
            .map_err(|e| format!("Failed to create record batch: {}", e))?;
        table.add_batch(batch)?;

        Ok(table)
    }
}

pub struct TokenizerPipeline {
    tokenizer: tokenizers::Tokenizer,
    config: TokenizerConfig,
}

impl TokenizerPipeline {
    pub fn from_file(path: &str, config: TokenizerConfig) -> Result<Self, String> {
        let tokenizer = tokenizers::Tokenizer::from_file(path)
            .map_err(|e| format!("Failed to load tokenizer from {}: {}", path, e))?;
        Ok(Self { tokenizer, config })
    }

    pub fn from_bytes(data: &[u8], config: TokenizerConfig) -> Result<Self, String> {
        let tokenizer = tokenizers::Tokenizer::from_bytes(data)
            .map_err(|e| format!("Failed to load tokenizer from bytes: {}", e))?;
        Ok(Self { tokenizer, config })
    }

    pub fn config(&self) -> &TokenizerConfig {
        &self.config
    }

    pub fn vocab_size(&self) -> usize {
        self.tokenizer.get_vocab_size(true)
    }

    pub fn token_to_id(&self, token: &str) -> Option<u32> {
        self.tokenizer.token_to_id(token)
    }

    pub fn id_to_token(&self, id: u32) -> Option<String> {
        self.tokenizer.id_to_token(id)
    }

    pub fn encode(&self, text: &str) -> Result<BatchEncoding, String> {
        let encoding = self.tokenizer.encode(text, self.config.add_special_tokens)
            .map_err(|e| format!("Encoding failed: {}", e))?;

        let ids = encoding.get_ids().to_vec();
        let attention_mask: Vec<u8> = encoding.get_attention_mask().iter().map(|&x| x as u8).collect();
        let special_tokens_mask: Vec<u8> = encoding.get_special_tokens_mask().iter().map(|&x| x as u8).collect();
        let length = ids.len();

        Ok(BatchEncoding {
            input_ids: vec![ids],
            attention_mask: vec![attention_mask],
            token_type_ids: None,
            special_tokens_mask: Some(vec![special_tokens_mask]),
            length: vec![length],
            overflow_to_sample_mapping: None,
            metadata: HashMap::new(),
        })
    }

    pub fn encode_batch(
        &self,
        texts: &[String],
        _pairs: Option<&[String]>,
    ) -> Result<BatchEncoding, String> {
        if texts.is_empty() {
            return Err("No texts provided for encoding".to_string());
        }

        let mut all_input_ids: Vec<Vec<u32>> = Vec::with_capacity(texts.len());
        let mut all_attention_mask: Vec<Vec<u8>> = Vec::with_capacity(texts.len());
        let mut all_special_tokens_mask: Vec<Vec<u8>> = Vec::with_capacity(texts.len());
        let mut all_length: Vec<usize> = Vec::with_capacity(texts.len());

        for text in texts {
            let encoding = self.tokenizer.encode(text.as_str(), self.config.add_special_tokens)
                .map_err(|e| format!("Encoding failed for text: {}", e))?;

            let ids = encoding.get_ids().to_vec();
            all_length.push(ids.len());
            all_input_ids.push(ids);
            all_attention_mask.push(encoding.get_attention_mask().iter().map(|&x| x as u8).collect());
            all_special_tokens_mask.push(encoding.get_special_tokens_mask().iter().map(|&x| x as u8).collect());
        }

        if self.config.padding != PaddingStrategy::DoNotPad {
            let max_len = match self.config.padding {
                PaddingStrategy::MaxLength => self.config.max_length.min(
                    all_input_ids.iter().map(|ids| ids.len()).max().unwrap_or(0)
                ),
                PaddingStrategy::Fixed(n) => n,
                _ => all_input_ids.iter().map(|ids| ids.len()).max().unwrap_or(0),
            };

            let pad_id = self.get_pad_token_id();

            for (ids, mask) in all_input_ids.iter_mut().zip(all_attention_mask.iter_mut()) {
                if self.config.truncation && ids.len() > max_len {
                    ids.truncate(max_len);
                    mask.truncate(max_len);
                }
                while ids.len() < max_len {
                    ids.push(pad_id);
                    mask.push(0);
                }
            }
        }

        Ok(BatchEncoding {
            input_ids: all_input_ids,
            attention_mask: all_attention_mask,
            token_type_ids: None,
            special_tokens_mask: Some(all_special_tokens_mask),
            length: all_length,
            overflow_to_sample_mapping: None,
            metadata: HashMap::new(),
        })
    }

    pub fn encode_from_arrow(
        &self,
        table: &ArrowTable,
        text_column: &str,
        pair_column: Option<&str>,
    ) -> Result<BatchEncoding, String> {
        let texts = table.get_column_as_strings(text_column)?;

        let pairs = match pair_column {
            Some(col) => Some(table.get_column_as_strings(col)?),
            None => None,
        };

        self.encode_batch(&texts, pairs.as_deref())
    }

    pub fn decode(&self, ids: &[u32], skip_special_tokens: bool) -> Result<String, String> {
        self.tokenizer.decode(ids, skip_special_tokens)
            .map_err(|e| format!("Decoding failed: {}", e))
    }

    pub fn decode_batch(&self, batch_ids: &[Vec<u32>], skip_special_tokens: bool) -> Result<Vec<String>, String> {
        batch_ids.iter()
            .map(|ids| self.decode(ids, skip_special_tokens))
            .collect()
    }

    fn get_pad_token_id(&self) -> u32 {
        if let Some(padding) = self.tokenizer.get_padding() {
            return padding.pad_id;
        }
        0
    }

    pub fn get_special_tokens(&self) -> SpecialTokens {
        let padding = self.tokenizer.get_padding();
        let added_tokens = self.tokenizer.get_added_tokens_decoder();

        let mut special = SpecialTokens::default();

        if let Some(p) = padding {
            special.pad_token = Some(p.pad_token.clone());
            special.pad_token_id = Some(p.pad_id);
        }

        for (id, token) in added_tokens.iter() {
            let tok_str = token.content.as_str();
            if token.special {
                match tok_str {
                    t if t == "<s>" || t == "<bos>" || t == "[CLS]" => {
                        special.bos_token = Some(t.to_string());
                        special.bos_token_id = Some(*id);
                    }
                    t if t == "</s>" || t == "<eos>" || t == "[SEP]" => {
                        if special.eos_token.is_none() {
                            special.eos_token = Some(t.to_string());
                            special.eos_token_id = Some(*id);
                        }
                        if special.sep_token.is_none() && (t == "[SEP]" || t == "</s>") {
                            special.sep_token = Some(t.to_string());
                            special.sep_token_id = Some(*id);
                        }
                    }
                    t if t == "<unk>" || t == "[UNK]" => {
                        special.unk_token = Some(t.to_string());
                        special.unk_token_id = Some(*id);
                    }
                    t if t == "<mask>" || t == "[MASK]" => {
                        special.mask_token = Some(t.to_string());
                        special.mask_token_id = Some(*id);
                    }
                    _ => {}
                }
            }
        }

        special
    }

    pub fn tokenizer_info(&self) -> TokenizerInfo {
        let special = self.get_special_tokens();
        TokenizerInfo {
            vocab_size: self.vocab_size(),
            max_length: self.config.max_length,
            truncation: self.config.truncation,
            padding: self.config.padding,
            special_tokens: special,
            model_type: "tokenizers".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpecialTokens {
    pub pad_token: Option<String>,
    pub pad_token_id: Option<u32>,
    pub bos_token: Option<String>,
    pub bos_token_id: Option<u32>,
    pub eos_token: Option<String>,
    pub eos_token_id: Option<u32>,
    pub unk_token: Option<String>,
    pub unk_token_id: Option<u32>,
    pub sep_token: Option<String>,
    pub sep_token_id: Option<u32>,
    pub cls_token: Option<String>,
    pub cls_token_id: Option<u32>,
    pub mask_token: Option<String>,
    pub mask_token_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizerInfo {
    pub vocab_size: usize,
    pub max_length: usize,
    pub truncation: bool,
    pub padding: PaddingStrategy,
    pub special_tokens: SpecialTokens,
    pub model_type: String,
}

pub fn estimate_token_count(text: &str, chars_per_token: f64) -> usize {
    let char_count = text.chars().count();
    (char_count as f64 / chars_per_token).ceil() as usize
}

pub fn estimate_batch_token_count(texts: &[String], chars_per_token: f64) -> usize {
    texts.iter()
        .map(|t| estimate_token_count(t, chars_per_token))
        .sum()
}
