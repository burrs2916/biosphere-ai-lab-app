use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::data::arrow_table::ArrowTable;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurationConfig {
    pub pii_detection: PiiDetectionConfig,
    pub toxicity_filter: ToxicityFilterConfig,
    pub language_detection: LanguageDetectionConfig,
    pub quality_filter: QualityFilterConfig,
    pub deduplication: DeduplicationConfig,
    pub length_filter: LengthFilterConfig,
}

impl Default for CurationConfig {
    fn default() -> Self {
        Self {
            pii_detection: PiiDetectionConfig::default(),
            toxicity_filter: ToxicityFilterConfig::default(),
            language_detection: LanguageDetectionConfig::default(),
            quality_filter: QualityFilterConfig::default(),
            deduplication: DeduplicationConfig::default(),
            length_filter: LengthFilterConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiDetectionConfig {
    pub enabled: bool,
    pub detect_emails: bool,
    pub detect_phones: bool,
    pub detect_ips: bool,
    pub detect_ssn: bool,
    pub detect_credit_cards: bool,
    pub detect_addresses: bool,
    pub mask_pii: bool,
    pub mask_char: String,
}

impl Default for PiiDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            detect_emails: true,
            detect_phones: true,
            detect_ips: true,
            detect_ssn: true,
            detect_credit_cards: true,
            detect_addresses: false,
            mask_pii: true,
            mask_char: "*".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToxicityFilterConfig {
    pub enabled: bool,
    pub threshold: f64,
    pub categories: Vec<String>,
    pub action: ToxicityAction,
}

impl Default for ToxicityFilterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 0.7,
            categories: vec![
                "toxic".to_string(),
                "severe_toxic".to_string(),
                "obscene".to_string(),
                "threat".to_string(),
                "insult".to_string(),
                "identity_hate".to_string(),
            ],
            action: ToxicityAction::Filter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToxicityAction {
    Filter,
    Flag,
    Anonymize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDetectionConfig {
    pub enabled: bool,
    pub target_languages: Vec<String>,
    pub min_confidence: f64,
    pub action: LanguageAction,
}

impl Default for LanguageDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            target_languages: vec!["en".to_string(), "zh".to_string()],
            min_confidence: 0.8,
            action: LanguageAction::Filter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageAction {
    Filter,
    Flag,
    Keep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityFilterConfig {
    pub enabled: bool,
    pub min_text_length: usize,
    pub max_text_length: usize,
    pub min_word_count: usize,
    pub max_repetition_ratio: f64,
    pub max_special_char_ratio: f64,
    pub min_alphanumeric_ratio: f64,
    pub max_url_ratio: f64,
    pub require_complete_sentences: bool,
    pub min_perplexity: Option<f64>,
    pub max_perplexity: Option<f64>,
}

impl Default for QualityFilterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_text_length: 50,
            max_text_length: 100_000,
            min_word_count: 10,
            max_repetition_ratio: 0.3,
            max_special_char_ratio: 0.3,
            min_alphanumeric_ratio: 0.5,
            max_url_ratio: 0.2,
            require_complete_sentences: true,
            min_perplexity: None,
            max_perplexity: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationConfig {
    pub enabled: bool,
    pub method: DedupMethod,
    pub ngram_size: usize,
    pub similarity_threshold: f64,
    pub minhash_num_perm: usize,
}

impl Default for DeduplicationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            method: DedupMethod::MinHash,
            ngram_size: 5,
            similarity_threshold: 0.8,
            minhash_num_perm: 128,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DedupMethod {
    Exact,
    Fuzzy,
    MinHash,
    SimHash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthFilterConfig {
    pub enabled: bool,
    pub min_chars: usize,
    pub max_chars: usize,
    pub min_words: usize,
    pub max_words: usize,
}

impl Default for LengthFilterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_chars: 10,
            max_chars: 1_000_000,
            min_words: 3,
            max_words: 100_000,
        }
    }
}

pub struct DataCurator {
    config: CurationConfig,
}

impl DataCurator {
    pub fn new(config: CurationConfig) -> Self {
        Self { config }
    }

    pub fn curate_dataset(&self, table: &ArrowTable, text_column: &str) -> Result<CurationReport, String> {
        let texts = table.get_column_as_strings(text_column)?;
        let total = texts.len();
        let mut report = CurationReport::new(total);

        for (idx, text) in texts.iter().enumerate() {
            let mut sample_report = SampleCurationResult {
                index: idx,
                passed: true,
                pii_detected: vec![],
                toxicity_score: 0.0,
                language: None,
                language_confidence: 0.0,
                quality_score: 1.0,
                is_duplicate: false,
                issues: vec![],
            };

            if self.config.pii_detection.enabled {
                let pii_result = self.detect_pii(text);
                if !pii_result.is_empty() {
                    sample_report.pii_detected = pii_result;
                    sample_report.passed = false;
                    sample_report.issues.push("pii_detected".to_string());
                    report.pii_filtered += 1;
                }
            }

            if self.config.toxicity_filter.enabled {
                let tox_score = self.estimate_toxicity(text);
                sample_report.toxicity_score = tox_score;
                if tox_score > self.config.toxicity_filter.threshold {
                    sample_report.passed = false;
                    sample_report.issues.push("toxic_content".to_string());
                    report.toxicity_filtered += 1;
                }
            }

            if self.config.language_detection.enabled {
                let (lang, conf) = self.detect_language(text);
                sample_report.language = Some(lang.clone());
                sample_report.language_confidence = conf;
                if conf < self.config.language_detection.min_confidence
                    || !self.config.language_detection.target_languages.contains(&lang)
                {
                    sample_report.passed = false;
                    sample_report.issues.push(format!("language_mismatch:{}", lang));
                    report.language_filtered += 1;
                }
            }

            if self.config.quality_filter.enabled {
                let quality = self.assess_quality(text);
                sample_report.quality_score = quality.score;
                if !quality.passed {
                    sample_report.passed = false;
                    for issue in &quality.issues {
                        sample_report.issues.push(issue.clone());
                    }
                    report.quality_filtered += 1;
                }
            }

            if self.config.length_filter.enabled {
                let char_count = text.chars().count();
                let word_count = text.split_whitespace().count();
                if char_count < self.config.length_filter.min_chars
                    || char_count > self.config.length_filter.max_chars
                    || word_count < self.config.length_filter.min_words
                    || word_count > self.config.length_filter.max_words
                {
                    sample_report.passed = false;
                    sample_report.issues.push("length_out_of_range".to_string());
                    report.length_filtered += 1;
                }
            }

            if sample_report.passed {
                report.passed += 1;
            }

            report.samples.push(sample_report);
        }

        if self.config.deduplication.enabled {
            let dup_result = self.detect_duplicates(&texts);
            report.duplicates_removed = dup_result.duplicates_removed;
            report.duplicate_clusters = dup_result.clusters.len();

            for cluster in &dup_result.clusters {
                for &idx in &cluster.indices[1..] {
                    if idx < report.samples.len() {
                        report.samples[idx].is_duplicate = true;
                        report.samples[idx].passed = false;
                        report.samples[idx].issues.push("duplicate".to_string());
                    }
                }
            }
        }

        report.compute_statistics();
        Ok(report)
    }

    fn detect_pii(&self, text: &str) -> Vec<String> {
        let mut detected = Vec::new();

        if self.config.pii_detection.detect_emails {
            if PII_PATTERNS.email.is_match(text) {
                detected.push("email".to_string());
            }
        }

        if self.config.pii_detection.detect_phones {
            if PII_PATTERNS.phone.is_match(text) {
                detected.push("phone".to_string());
            }
        }

        if self.config.pii_detection.detect_ips {
            if PII_PATTERNS.ip_address.is_match(text) {
                detected.push("ip_address".to_string());
            }
        }

        if self.config.pii_detection.detect_ssn {
            if PII_PATTERNS.ssn.is_match(text) {
                detected.push("ssn".to_string());
            }
        }

        if self.config.pii_detection.detect_credit_cards {
            if PII_PATTERNS.credit_card.is_match(text) {
                detected.push("credit_card".to_string());
            }
        }

        detected
    }

    pub fn mask_pii(&self, text: &str) -> String {
        if !self.config.pii_detection.mask_pii {
            return text.to_string();
        }

        let mask_char = &self.config.pii_detection.mask_char;
        let mut result = text.to_string();

        if self.config.pii_detection.detect_emails {
            result = PII_PATTERNS.email.replace_all(&result, |_: &regex::Captures| {
                format!("{}@{}.***", mask_char.repeat(5), mask_char.repeat(5))
            }).to_string();
        }

        if self.config.pii_detection.detect_phones {
            result = PII_PATTERNS.phone.replace_all(&result, |_: &regex::Captures| {
                format!("{}-{}-{}", mask_char.repeat(3), mask_char.repeat(3), mask_char.repeat(4))
            }).to_string();
        }

        if self.config.pii_detection.detect_ips {
            result = PII_PATTERNS.ip_address.replace_all(&result, |_: &regex::Captures| {
                format!("{}.***.***.***", mask_char.repeat(3))
            }).to_string();
        }

        if self.config.pii_detection.detect_credit_cards {
            result = PII_PATTERNS.credit_card.replace_all(&result, |_: &regex::Captures| {
                format!("****-****-****-{}", mask_char.repeat(4))
            }).to_string();
        }

        result
    }

    fn estimate_toxicity(&self, text: &str) -> f64 {
        let lower = text.to_lowercase();
        let mut score = 0.0;
        let mut matches = 0usize;

        for pattern in TOXIC_PATTERNS {
            if lower.contains(pattern) {
                matches += 1;
            }
        }

        if matches > 0 {
            score = (matches as f64 / TOXIC_PATTERNS.len() as f64).min(1.0);
        }

        let word_count = text.split_whitespace().count().max(1);
        let caps_ratio = text.chars().filter(|c| c.is_uppercase()).count() as f64 / text.chars().count().max(1) as f64;
        if caps_ratio > 0.5 && word_count > 5 {
            score = (score + 0.2).min(1.0);
        }

        let exclamation_ratio = text.chars().filter(|&c| c == '!').count() as f64 / text.chars().count().max(1) as f64;
        if exclamation_ratio > 0.05 {
            score = (score + 0.1).min(1.0);
        }

        score
    }

    fn detect_language(&self, text: &str) -> (String, f64) {
        let mut scores: HashMap<&str, f64> = HashMap::new();

        for (lang, patterns) in LANGUAGE_PATTERNS.iter() {
            let mut score = 0.0;
            let lower = text.to_lowercase();
            for pattern in *patterns {
                if lower.contains(pattern) {
                    score += 1.0;
                }
            }
            let text_len = text.len().max(1) as f64;
            let normalized = score / (text_len / 10.0).max(1.0);
            scores.insert(*lang, normalized.min(1.0));
        }

        let mut best_lang = "unknown".to_string();
        let mut best_score = 0.0;

        for (lang, score) in &scores {
            if *score > best_score {
                best_score = *score;
                best_lang = lang.to_string();
            }
        }

        let char_ranges = analyze_char_ranges(text);
        if char_ranges.cjk_ratio > 0.3 {
            if best_lang != "zh" {
                best_score = (best_score + char_ranges.cjk_ratio).min(1.0);
                best_lang = "zh".to_string();
            }
        }

        (best_lang, best_score)
    }

    fn assess_quality(&self, text: &str) -> QualityAssessment {
        let cfg = &self.config.quality_filter;
        let mut issues = Vec::new();
        let mut score: f64 = 1.0;

        let char_count = text.chars().count();
        if char_count < cfg.min_text_length {
            issues.push("text_too_short".to_string());
            score -= 0.3;
        }
        if char_count > cfg.max_text_length {
            issues.push("text_too_long".to_string());
            score -= 0.2;
        }

        let word_count = text.split_whitespace().count();
        if word_count < cfg.min_word_count {
            issues.push("too_few_words".to_string());
            score -= 0.3;
        }

        let repetition_ratio = compute_repetition_ratio(text);
        if repetition_ratio > cfg.max_repetition_ratio {
            issues.push("high_repetition".to_string());
            score -= 0.3;
        }

        let special_char_ratio = text.chars().filter(|c| !c.is_alphanumeric() && !c.is_whitespace()).count() as f64
            / char_count.max(1) as f64;
        if special_char_ratio > cfg.max_special_char_ratio {
            issues.push("high_special_chars".to_string());
            score -= 0.2;
        }

        let alphanumeric_ratio = text.chars().filter(|c| c.is_alphanumeric()).count() as f64
            / char_count.max(1) as f64;
        if alphanumeric_ratio < cfg.min_alphanumeric_ratio {
            issues.push("low_alphanumeric".to_string());
            score -= 0.2;
        }

        let url_count = PII_PATTERNS.url.find_iter(text).count();
        let url_ratio = url_count as f64 / word_count.max(1) as f64;
        if url_ratio > cfg.max_url_ratio {
            issues.push("too_many_urls".to_string());
            score -= 0.2;
        }

        if cfg.require_complete_sentences {
            let sentence_endings = text.matches(|c: char| c == '.' || c == '!' || c == '?' || c == '。').count();
            if sentence_endings == 0 && word_count > 20 {
                issues.push("no_sentence_endings".to_string());
                score -= 0.1;
            }
        }

        let score = score.max(0.0);
        let passed = issues.is_empty();

        QualityAssessment { score, passed, issues }
    }

    fn detect_duplicates(&self, texts: &[String]) -> DedupResult {
        match self.config.deduplication.method {
            DedupMethod::Exact => self.exact_dedup(texts),
            DedupMethod::Fuzzy => self.fuzzy_dedup(texts),
            DedupMethod::MinHash => self.minhash_dedup(texts),
            DedupMethod::SimHash => self.simhash_dedup(texts),
        }
    }

    fn exact_dedup(&self, texts: &[String]) -> DedupResult {
        let mut seen: HashMap<&str, usize> = HashMap::new();
        let mut clusters: Vec<DupCluster> = Vec::new();
        let mut removed = 0usize;

        for (idx, text) in texts.iter().enumerate() {
            if let Some(&first_idx) = seen.get(text.as_str()) {
                let cluster = clusters.iter_mut().find(|c| c.indices.contains(&first_idx));
                if let Some(cluster) = cluster {
                    cluster.indices.push(idx);
                } else {
                    clusters.push(DupCluster {
                        representative: first_idx,
                        indices: vec![first_idx, idx],
                        similarity: 1.0,
                    });
                }
                removed += 1;
            } else {
                seen.insert(text.as_str(), idx);
            }
        }

        DedupResult {
            duplicates_removed: removed,
            clusters,
        }
    }

    fn fuzzy_dedup(&self, texts: &[String]) -> DedupResult {
        let threshold = self.config.deduplication.similarity_threshold;
        let n = self.config.deduplication.ngram_size;
        let mut clusters: Vec<DupCluster> = Vec::new();
        let mut removed = 0usize;
        let mut processed = vec![false; texts.len()];

        for i in 0..texts.len() {
            if processed[i] {
                continue;
            }
            processed[i] = true;

            let mut cluster = DupCluster {
                representative: i,
                indices: vec![i],
                similarity: 1.0,
            };

            for j in (i + 1)..texts.len() {
                if processed[j] {
                    continue;
                }
                let sim = jaccard_similarity_ngram(&texts[i], &texts[j], n);
                if sim >= threshold {
                    cluster.indices.push(j);
                    cluster.similarity = cluster.similarity.min(sim);
                    processed[j] = true;
                    removed += 1;
                }
            }

            if cluster.indices.len() > 1 {
                clusters.push(cluster);
            }
        }

        DedupResult {
            duplicates_removed: removed,
            clusters,
        }
    }

    fn minhash_dedup(&self, texts: &[String]) -> DedupResult {
        let threshold = self.config.deduplication.similarity_threshold;
        let n = self.config.deduplication.ngram_size;
        let num_perm = self.config.deduplication.minhash_num_perm;

        let signatures: Vec<Vec<u64>> = texts.iter()
            .map(|text| compute_minhash_signature(text, n, num_perm))
            .collect();

        let mut clusters: Vec<DupCluster> = Vec::new();
        let mut removed = 0usize;
        let mut processed = vec![false; texts.len()];

        for i in 0..texts.len() {
            if processed[i] {
                continue;
            }
            processed[i] = true;

            let mut cluster = DupCluster {
                representative: i,
                indices: vec![i],
                similarity: 1.0,
            };

            for j in (i + 1)..texts.len() {
                if processed[j] {
                    continue;
                }
                let sim = minhash_similarity(&signatures[i], &signatures[j]);
                if sim >= threshold {
                    cluster.indices.push(j);
                    cluster.similarity = cluster.similarity.min(sim);
                    processed[j] = true;
                    removed += 1;
                }
            }

            if cluster.indices.len() > 1 {
                clusters.push(cluster);
            }
        }

        DedupResult {
            duplicates_removed: removed,
            clusters,
        }
    }

    fn simhash_dedup(&self, texts: &[String]) -> DedupResult {
        let threshold = self.config.deduplication.similarity_threshold;
        let n = self.config.deduplication.ngram_size;

        let hashes: Vec<u64> = texts.iter()
            .map(|text| compute_simhash(text, n))
            .collect();

        let mut clusters: Vec<DupCluster> = Vec::new();
        let mut removed = 0usize;
        let mut processed = vec![false; texts.len()];

        for i in 0..texts.len() {
            if processed[i] {
                continue;
            }
            processed[i] = true;

            let mut cluster = DupCluster {
                representative: i,
                indices: vec![i],
                similarity: 1.0,
            };

            for j in (i + 1)..texts.len() {
                if processed[j] {
                    continue;
                }
                let dist = hamming_distance(hashes[i], hashes[j]);
                let sim = 1.0 - (dist as f64 / 64.0);
                if sim >= threshold {
                    cluster.indices.push(j);
                    cluster.similarity = cluster.similarity.min(sim);
                    processed[j] = true;
                    removed += 1;
                }
            }

            if cluster.indices.len() > 1 {
                clusters.push(cluster);
            }
        }

        DedupResult {
            duplicates_removed: removed,
            clusters,
        }
    }
}

struct PiiPatterns {
    email: regex::Regex,
    phone: regex::Regex,
    ip_address: regex::Regex,
    ssn: regex::Regex,
    credit_card: regex::Regex,
    url: regex::Regex,
}

lazy_static::lazy_static! {
    static ref PII_PATTERNS: PiiPatterns = PiiPatterns {
        email: regex::Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap(),
        phone: regex::Regex::new(r"\b(\+?\d{1,3}[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap(),
        ip_address: regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap(),
        ssn: regex::Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
        credit_card: regex::Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b").unwrap(),
        url: regex::Regex::new(r"https?://[^\s]+").unwrap(),
    };
}

static TOXIC_PATTERNS: &[&str] = &[
    "kill yourself", "die in a fire", "i hate you",
    "stupid idiot", "shut up", "go away",
    "worthless", "pathetic", "disgusting",
    "moron", "imbecile", "degenerate",
];

static LANGUAGE_PATTERNS: &[(&str, &[&str])] = &[
    ("en", &["the", "and", "that", "have", "for", "with", "this", "from", "they", "will"]),
    ("zh", &["的", "是", "在", "了", "不", "和", "有", "我", "他", "这"]),
    ("ja", &["の", "に", "を", "は", "が", "と", "で", "た", "し", "て"]),
    ("ko", &["이", "가", "는", "을", "를", "에", "의", "로", "고", "다"]),
    ("fr", &["le", "la", "les", "des", "est", "que", "pas", "une", "dans", "pour"]),
    ("de", &["der", "die", "das", "und", "ist", "nicht", "ein", "eine", "von", "mit"]),
    ("es", &["que", "los", "las", "una", "por", "del", "con", "para", "como", "más"]),
    ("ru", &["и", "в", "не", "на", "что", "с", "это", "а", "по", "как"]),
    ("ar", &["في", "من", "على", "أن", "هذا", "الذي", "مع", "كان", "هذه", "ما"]),
];

fn analyze_char_ranges(text: &str) -> CharRangeAnalysis {
    let total = text.chars().count().max(1) as f64;
    let cjk = text.chars().filter(|c| {
        ('\u{4E00}'..='\u{9FFF}').contains(c)
            || ('\u{3040}'..='\u{309F}').contains(c)
            || ('\u{30A0}'..='\u{30FF}').contains(c)
            || ('\u{AC00}'..='\u{D7AF}').contains(c)
    }).count() as f64;

    let latin = text.chars().filter(|c| c.is_ascii_alphabetic()).count() as f64;
    let cyrillic = text.chars().filter(|c| ('\u{0400}'..='\u{04FF}').contains(c)).count() as f64;
    let arabic = text.chars().filter(|c| ('\u{0600}'..='\u{06FF}').contains(c)).count() as f64;

    CharRangeAnalysis {
        cjk_ratio: cjk / total,
        latin_ratio: latin / total,
        cyrillic_ratio: cyrillic / total,
        arabic_ratio: arabic / total,
    }
}

struct CharRangeAnalysis {
    cjk_ratio: f64,
    latin_ratio: f64,
    cyrillic_ratio: f64,
    arabic_ratio: f64,
}

fn compute_repetition_ratio(text: &str) -> f64 {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 4 {
        return 0.0;
    }

    let n = 4usize;
    let mut seen = std::collections::HashSet::new();
    let mut repeated = 0usize;

    for window in words.windows(n) {
        let key = window.join(" ");
        if !seen.insert(key) {
            repeated += 1;
        }
    }

    let total_windows = words.len().saturating_sub(n - 1).max(1);
    repeated as f64 / total_windows as f64
}

fn jaccard_similarity_ngram(a: &str, b: &str, n: usize) -> f64 {
    let a_ngrams: std::collections::HashSet<String> = ngrams(a, n).into_iter().collect();
    let b_ngrams: std::collections::HashSet<String> = ngrams(b, n).into_iter().collect();

    if a_ngrams.is_empty() && b_ngrams.is_empty() {
        return 1.0;
    }

    let intersection = a_ngrams.intersection(&b_ngrams).count();
    let union = a_ngrams.union(&b_ngrams).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}

fn ngrams(text: &str, n: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    chars.windows(n)
        .map(|w| w.iter().collect::<String>())
        .collect()
}

fn compute_minhash_signature(text: &str, ngram_size: usize, num_perm: usize) -> Vec<u64> {
    let ngrams: Vec<String> = ngrams(text, ngram_size);
    if ngrams.is_empty() {
        return vec![u64::MAX; num_perm];
    }

    let mut signature = vec![u64::MAX; num_perm];

    for ngram in &ngrams {
        let hash = hash_string(ngram);
        for i in 0..num_perm {
            let perm_hash = permute_hash(hash, i as u64);
            if perm_hash < signature[i] {
                signature[i] = perm_hash;
            }
        }
    }

    signature
}

fn minhash_similarity(a: &[u64], b: &[u64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let matches = a.iter().zip(b.iter()).filter(|(x, y)| x == y).count();
    matches as f64 / a.len() as f64
}

fn compute_simhash(text: &str, ngram_size: usize) -> u64 {
    let ngrams = ngrams(text, ngram_size);
    let mut counts = [0i64; 64];

    for ngram in &ngrams {
        let hash = hash_string(ngram);
        for i in 0..64 {
            if (hash >> i) & 1 == 1 {
                counts[i] += 1;
            } else {
                counts[i] -= 1;
            }
        }
    }

    let mut simhash: u64 = 0;
    for i in 0..64 {
        if counts[i] > 0 {
            simhash |= 1 << i;
        }
    }

    simhash
}

fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

fn hash_string(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

fn permute_hash(hash: u64, seed: u64) -> u64 {
    let a = hash.wrapping_mul(0x9E3779B97F4A7C15);
    let b = seed.wrapping_mul(0xBF58476D1CE4E5B9);
    a.wrapping_add(b).wrapping_mul(0x94D049BB133111EB)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurationReport {
    pub total_samples: usize,
    pub passed: usize,
    pub pii_filtered: usize,
    pub toxicity_filtered: usize,
    pub language_filtered: usize,
    pub quality_filtered: usize,
    pub length_filtered: usize,
    pub duplicates_removed: usize,
    pub duplicate_clusters: usize,
    pub pass_rate: f64,
    pub samples: Vec<SampleCurationResult>,
}

impl CurationReport {
    fn new(total: usize) -> Self {
        Self {
            total_samples: total,
            passed: 0,
            pii_filtered: 0,
            toxicity_filtered: 0,
            language_filtered: 0,
            quality_filtered: 0,
            length_filtered: 0,
            duplicates_removed: 0,
            duplicate_clusters: 0,
            pass_rate: 0.0,
            samples: Vec::with_capacity(total),
        }
    }

    fn compute_statistics(&mut self) {
        if self.total_samples > 0 {
            self.pass_rate = self.passed as f64 / self.total_samples as f64;
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "Curation Report: {}/{} passed ({:.1}%)\n\
             PII filtered: {}, Toxicity filtered: {}, Language filtered: {}\n\
             Quality filtered: {}, Length filtered: {}, Duplicates removed: {}",
            self.passed,
            self.total_samples,
            self.pass_rate * 100.0,
            self.pii_filtered,
            self.toxicity_filtered,
            self.language_filtered,
            self.quality_filtered,
            self.length_filtered,
            self.duplicates_removed,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleCurationResult {
    pub index: usize,
    pub passed: bool,
    pub pii_detected: Vec<String>,
    pub toxicity_score: f64,
    pub language: Option<String>,
    pub language_confidence: f64,
    pub quality_score: f64,
    pub is_duplicate: bool,
    pub issues: Vec<String>,
}

struct QualityAssessment {
    score: f64,
    passed: bool,
    issues: Vec<String>,
}

struct DedupResult {
    duplicates_removed: usize,
    clusters: Vec<DupCluster>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DupCluster {
    pub representative: usize,
    pub indices: Vec<usize>,
    pub similarity: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pii_detection_email() {
        let config = CurationConfig::default();
        let curator = DataCurator::new(config);
        let detected = curator.detect_pii("Contact me at john.doe@example.com");
        assert!(detected.contains(&"email".to_string()));
    }

    #[test]
    fn test_pii_detection_phone() {
        let config = CurationConfig::default();
        let curator = DataCurator::new(config);
        let detected = curator.detect_pii("Call me at 555-123-4567");
        assert!(detected.contains(&"phone".to_string()));
    }

    #[test]
    fn test_pii_detection_ip() {
        let config = CurationConfig::default();
        let curator = DataCurator::new(config);
        let detected = curator.detect_pii("Server at 192.168.1.1 is down");
        assert!(detected.contains(&"ip_address".to_string()));
    }

    #[test]
    fn test_pii_mask_email() {
        let config = CurationConfig::default();
        let curator = DataCurator::new(config);
        let masked = curator.mask_pii("Email: john@example.com");
        assert!(!masked.contains("john@example.com"));
        assert!(masked.contains("*****@*****.***"));
    }

    #[test]
    fn test_language_detection_english() {
        let config = CurationConfig::default();
        let curator = DataCurator::new(config);
        let (lang, conf) = curator.detect_language("The quick brown fox jumps over the lazy dog");
        assert_eq!(lang, "en");
        assert!(conf > 0.0);
    }

    #[test]
    fn test_language_detection_chinese() {
        let config = CurationConfig::default();
        let curator = DataCurator::new(config);
        let (lang, conf) = curator.detect_language("这是一个中文测试句子，用于检测语言");
        assert_eq!(lang, "zh");
        assert!(conf > 0.0);
    }

    #[test]
    fn test_quality_assessment_good_text() {
        let config = CurationConfig::default();
        let curator = DataCurator::new(config);
        let quality = curator.assess_quality(
            "This is a high quality text sample. It contains multiple sentences. \
             The content is well structured and informative. It should pass all quality checks."
        );
        assert!(quality.passed);
        assert!(quality.score > 0.8);
    }

    #[test]
    fn test_quality_assessment_short_text() {
        let config = CurationConfig::default();
        let curator = DataCurator::new(config);
        let quality = curator.assess_quality("too short");
        assert!(!quality.passed);
    }

    #[test]
    fn test_exact_dedup() {
        let config = CurationConfig::default();
        let curator = DataCurator::new(config);
        let texts = vec![
            "unique text one".to_string(),
            "duplicate text".to_string(),
            "unique text two".to_string(),
            "duplicate text".to_string(),
        ];
        let result = curator.exact_dedup(&texts);
        assert_eq!(result.duplicates_removed, 1);
    }

    #[test]
    fn test_jaccard_similarity() {
        let sim = jaccard_similarity_ngram("hello world", "hello world", 3);
        assert!((sim - 1.0).abs() < 0.001);

        let sim = jaccard_similarity_ngram("hello world", "goodbye world", 3);
        assert!(sim < 1.0);
    }

    #[test]
    fn test_minhash_signature() {
        let sig1 = compute_minhash_signature("hello world this is a test", 3, 128);
        let sig2 = compute_minhash_signature("hello world this is a test", 3, 128);
        assert_eq!(sig1.len(), 128);
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_simhash() {
        let h1 = compute_simhash("hello world", 3);
        let h2 = compute_simhash("hello world", 3);
        assert_eq!(h1, h2);
        assert_eq!(hamming_distance(h1, h2), 0);
    }
}
