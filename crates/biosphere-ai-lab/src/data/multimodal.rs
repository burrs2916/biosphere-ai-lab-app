use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimodalAnalysisReport {
    pub dataset_id: String,
    pub modality: DataModality,
    pub total_samples: usize,
    pub image_analysis: Option<ImageAnalysisReport>,
    pub text_analysis: Option<TextAnalysisReport>,
    pub overall_quality_score: f64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataModality {
    Image,
    Text,
    ImageText,
    Unknown,
}

impl std::fmt::Display for DataModality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Image => write!(f, "image"),
            Self::Text => write!(f, "text"),
            Self::ImageText => write!(f, "image_text"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAnalysisReport {
    pub total_images: usize,
    pub resolution_stats: ResolutionStats,
    pub aspect_ratio_distribution: Vec<AspectRatioBucket>,
    pub file_size_stats: FileSizeStats,
    pub color_analysis: Option<ColorAnalysis>,
    pub format_distribution: Vec<FormatCount>,
    pub quality_issues: Vec<ImageQualityIssue>,
    pub image_quality_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionStats {
    pub min_width: usize,
    pub max_width: usize,
    pub avg_width: f64,
    pub median_width: f64,
    pub min_height: usize,
    pub max_height: usize,
    pub avg_height: f64,
    pub median_height: f64,
    pub total_pixels_min: usize,
    pub total_pixels_max: usize,
    pub total_pixels_avg: f64,
    pub resolution_buckets: Vec<ResolutionBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionBucket {
    pub label: String,
    pub min_pixels: usize,
    pub max_pixels: Option<usize>,
    pub count: usize,
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AspectRatioBucket {
    pub label: String,
    pub min_ratio: f64,
    pub max_ratio: f64,
    pub count: usize,
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSizeStats {
    pub min_bytes: u64,
    pub max_bytes: u64,
    pub avg_bytes: f64,
    pub median_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorAnalysis {
    pub grayscale_ratio: f64,
    pub avg_color_channels: f64,
    pub color_diversity_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatCount {
    pub format: String,
    pub count: usize,
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageQualityIssue {
    pub issue_type: String,
    pub count: usize,
    pub severity: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAnalysisReport {
    pub total_texts: usize,
    pub length_stats: TextLengthStats,
    pub token_stats: TokenStats,
    pub language_distribution: Vec<LanguageCount>,
    pub vocabulary_stats: VocabularyStats,
    pub ngram_diversity: NgramDiversity,
    pub quality_issues: Vec<TextQualityIssue>,
    pub text_quality_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextLengthStats {
    pub min_chars: usize,
    pub max_chars: usize,
    pub avg_chars: f64,
    pub median_chars: f64,
    pub min_words: usize,
    pub max_words: usize,
    pub avg_words: f64,
    pub median_words: f64,
    pub length_buckets: Vec<LengthBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthBucket {
    pub label: String,
    pub min_chars: usize,
    pub max_chars: Option<usize>,
    pub count: usize,
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStats {
    pub total_tokens: usize,
    pub unique_tokens: usize,
    pub avg_tokens_per_sample: f64,
    pub token_length_distribution: Vec<TokenLengthBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLengthBucket {
    pub label: String,
    pub min_tokens: usize,
    pub max_tokens: Option<usize>,
    pub count: usize,
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageCount {
    pub language: String,
    pub count: usize,
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabularyStats {
    pub total_words: usize,
    pub unique_words: usize,
    pub type_token_ratio: f64,
    pub hapax_legomena: usize,
    pub hapax_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NgramDiversity {
    pub bigram_diversity: f64,
    pub trigram_diversity: f64,
    pub repetition_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextQualityIssue {
    pub issue_type: String,
    pub count: usize,
    pub severity: String,
    pub description: String,
}

pub struct MultimodalAnalyzer;

impl MultimodalAnalyzer {
    pub fn analyze_images(
        dataset_id: &str,
        image_metadata: &[ImageMetadata],
    ) -> MultimodalAnalysisReport {
        let total = image_metadata.len();
        let resolution_stats = Self::compute_resolution_stats(image_metadata);
        let aspect_ratios = Self::compute_aspect_ratios(image_metadata);
        let file_size_stats = Self::compute_file_size_stats(image_metadata);
        let format_dist = Self::compute_format_distribution(image_metadata);
        let quality_issues = Self::detect_image_quality_issues(image_metadata, &resolution_stats);

        let image_score = Self::compute_image_quality_score(&quality_issues, &resolution_stats, total);

        let image_report = ImageAnalysisReport {
            total_images: total,
            resolution_stats,
            aspect_ratio_distribution: aspect_ratios,
            file_size_stats,
            color_analysis: None,
            format_distribution: format_dist,
            quality_issues,
            image_quality_score: image_score,
        };

        let recommendations = Self::generate_image_recommendations(&image_report);

        MultimodalAnalysisReport {
            dataset_id: dataset_id.to_string(),
            modality: DataModality::Image,
            total_samples: total,
            image_analysis: Some(image_report),
            text_analysis: None,
            overall_quality_score: image_score,
            recommendations,
        }
    }

    pub fn analyze_texts(
        dataset_id: &str,
        texts: &[String],
    ) -> MultimodalAnalysisReport {
        let total = texts.len();
        let length_stats = Self::compute_text_length_stats(texts);
        let token_stats = Self::compute_token_stats(texts);
        let vocab_stats = Self::compute_vocabulary_stats(texts);
        let ngram_div = Self::compute_ngram_diversity(texts);
        let quality_issues = Self::detect_text_quality_issues(texts, &length_stats);

        let text_score = Self::compute_text_quality_score(&quality_issues, &length_stats, &vocab_stats, total);

        let text_report = TextAnalysisReport {
            total_texts: total,
            length_stats,
            token_stats,
            language_distribution: Vec::new(),
            vocabulary_stats: vocab_stats,
            ngram_diversity: ngram_div,
            quality_issues,
            text_quality_score: text_score,
        };

        let recommendations = Self::generate_text_recommendations(&text_report);

        MultimodalAnalysisReport {
            dataset_id: dataset_id.to_string(),
            modality: DataModality::Text,
            total_samples: total,
            image_analysis: None,
            text_analysis: Some(text_report),
            overall_quality_score: text_score,
            recommendations,
        }
    }

    fn compute_resolution_stats(images: &[ImageMetadata]) -> ResolutionStats {
        if images.is_empty() {
            return ResolutionStats {
                min_width: 0, max_width: 0, avg_width: 0.0, median_width: 0.0,
                min_height: 0, max_height: 0, avg_height: 0.0, median_height: 0.0,
                total_pixels_min: 0, total_pixels_max: 0, total_pixels_avg: 0.0,
                resolution_buckets: Vec::new(),
            };
        }

        let widths: Vec<usize> = images.iter().map(|i| i.width).collect();
        let heights: Vec<usize> = images.iter().map(|i| i.height).collect();
        let pixels: Vec<usize> = images.iter().map(|i| i.width * i.height).collect();

        let mut sorted_w = widths.clone();
        sorted_w.sort();
        let mut sorted_h = heights.clone();
        sorted_h.sort();
        let mut sorted_p = pixels.clone();
        sorted_p.sort();

        let n = images.len();

        let buckets = vec![
            ResolutionBucket {
                label: "< 64x64".to_string(),
                min_pixels: 0,
                max_pixels: Some(64 * 64),
                count: pixels.iter().filter(|&&p| p < 64 * 64).count(),
                ratio: pixels.iter().filter(|&&p| p < 64 * 64).count() as f64 / n as f64,
            },
            ResolutionBucket {
                label: "64x64 - 256x256".to_string(),
                min_pixels: 64 * 64,
                max_pixels: Some(256 * 256),
                count: pixels.iter().filter(|&&p| p >= 64 * 64 && p < 256 * 256).count(),
                ratio: pixels.iter().filter(|&&p| p >= 64 * 64 && p < 256 * 256).count() as f64 / n as f64,
            },
            ResolutionBucket {
                label: "256x256 - 512x512".to_string(),
                min_pixels: 256 * 256,
                max_pixels: Some(512 * 512),
                count: pixels.iter().filter(|&&p| p >= 256 * 256 && p < 512 * 512).count(),
                ratio: pixels.iter().filter(|&&p| p >= 256 * 256 && p < 512 * 512).count() as f64 / n as f64,
            },
            ResolutionBucket {
                label: "512x512 - 1024x1024".to_string(),
                min_pixels: 512 * 512,
                max_pixels: Some(1024 * 1024),
                count: pixels.iter().filter(|&&p| p >= 512 * 512 && p < 1024 * 1024).count(),
                ratio: pixels.iter().filter(|&&p| p >= 512 * 512 && p < 1024 * 1024).count() as f64 / n as f64,
            },
            ResolutionBucket {
                label: "> 1024x1024".to_string(),
                min_pixels: 1024 * 1024,
                max_pixels: None,
                count: pixels.iter().filter(|&&p| p >= 1024 * 1024).count(),
                ratio: pixels.iter().filter(|&&p| p >= 1024 * 1024).count() as f64 / n as f64,
            },
        ];

        ResolutionStats {
            min_width: *widths.iter().min().unwrap_or(&0),
            max_width: *widths.iter().max().unwrap_or(&0),
            avg_width: widths.iter().sum::<usize>() as f64 / n as f64,
            median_width: sorted_w[n / 2] as f64,
            min_height: *heights.iter().min().unwrap_or(&0),
            max_height: *heights.iter().max().unwrap_or(&0),
            avg_height: heights.iter().sum::<usize>() as f64 / n as f64,
            median_height: sorted_h[n / 2] as f64,
            total_pixels_min: *pixels.iter().min().unwrap_or(&0),
            total_pixels_max: *pixels.iter().max().unwrap_or(&0),
            total_pixels_avg: pixels.iter().sum::<usize>() as f64 / n as f64,
            resolution_buckets: buckets,
        }
    }

    fn compute_aspect_ratios(images: &[ImageMetadata]) -> Vec<AspectRatioBucket> {
        let n = images.len();
        if n == 0 {
            return Vec::new();
        }

        let ratios: Vec<f64> = images.iter()
            .map(|i| if i.height > 0 { i.width as f64 / i.height as f64 } else { 1.0 })
            .collect();

        let buckets_def = vec![
            ("竖屏 (< 3:4)", 0.0, 0.75),
            ("近方形 (3:4 - 4:3)", 0.75, 1.33),
            ("横屏 (4:3 - 16:9)", 1.33, 1.78),
            ("宽屏 (> 16:9)", 1.78, f64::MAX),
        ];

        buckets_def.iter().map(|(label, min_r, max_r)| {
            let count = ratios.iter().filter(|&&r| r >= *min_r && r < *max_r).count();
            AspectRatioBucket {
                label: label.to_string(),
                min_ratio: *min_r,
                max_ratio: *max_r,
                count,
                ratio: count as f64 / n as f64,
            }
        }).collect()
    }

    fn compute_file_size_stats(images: &[ImageMetadata]) -> FileSizeStats {
        if images.is_empty() {
            return FileSizeStats {
                min_bytes: 0, max_bytes: 0, avg_bytes: 0.0, median_bytes: 0, total_bytes: 0,
            };
        }

        let mut sizes: Vec<u64> = images.iter().map(|i| i.file_size_bytes).collect();
        sizes.sort();

        let n = sizes.len();
        FileSizeStats {
            min_bytes: sizes[0],
            max_bytes: sizes[n - 1],
            avg_bytes: sizes.iter().sum::<u64>() as f64 / n as f64,
            median_bytes: sizes[n / 2],
            total_bytes: sizes.iter().sum(),
        }
    }

    fn compute_format_distribution(images: &[ImageMetadata]) -> Vec<FormatCount> {
        let n = images.len();
        if n == 0 {
            return Vec::new();
        }

        let mut counts: HashMap<&str, usize> = HashMap::new();
        for img in images {
            *counts.entry(&img.format).or_insert(0) += 1;
        }

        let mut result: Vec<FormatCount> = counts.iter()
            .map(|(fmt, &count)| FormatCount {
                format: fmt.to_string(),
                count,
                ratio: count as f64 / n as f64,
            })
            .collect();
        result.sort_by(|a, b| b.count.cmp(&a.count));
        result
    }

    fn detect_image_quality_issues(
        images: &[ImageMetadata],
        res_stats: &ResolutionStats,
    ) -> Vec<ImageQualityIssue> {
        let n = images.len();
        if n == 0 {
            return Vec::new();
        }

        let mut issues = Vec::new();

        let tiny_count = images.iter()
            .filter(|i| i.width * i.height < 64 * 64)
            .count();
        if tiny_count > 0 {
            issues.push(ImageQualityIssue {
                issue_type: "tiny_resolution".to_string(),
                count: tiny_count,
                severity: if tiny_count as f64 / n as f64 > 0.1 { "high" } else { "medium" }.to_string(),
                description: format!("{} 张图片分辨率极低 (< 64x64)，可能无法用于训练", tiny_count),
            });
        }

        let res_variance = if res_stats.avg_width > 0.0 {
            let widths: Vec<f64> = images.iter().map(|i| i.width as f64).collect();
            let mean = widths.iter().sum::<f64>() / n as f64;
            let variance = widths.iter().map(|w| (w - mean).powi(2)).sum::<f64>() / n as f64;
            variance.sqrt() / mean
        } else {
            0.0
        };

        if res_variance > 0.5 {
            issues.push(ImageQualityIssue {
                issue_type: "high_resolution_variance".to_string(),
                count: n,
                severity: "medium".to_string(),
                description: format!(
                    "分辨率差异较大（CV={:.1}），建议统一resize到固定尺寸",
                    res_variance
                ),
            });
        }

        let zero_size = images.iter().filter(|i| i.file_size_bytes == 0).count();
        if zero_size > 0 {
            issues.push(ImageQualityIssue {
                issue_type: "zero_size_files".to_string(),
                count: zero_size,
                severity: "high".to_string(),
                description: format!("{} 个文件大小为0，可能是损坏文件", zero_size),
            });
        }

        issues
    }

    fn compute_image_quality_score(
        issues: &[ImageQualityIssue],
        _res_stats: &ResolutionStats,
        total: usize,
    ) -> f64 {
        if total == 0 {
            return 0.0;
        }

        let mut penalty = 0.0;
        for issue in issues {
            let weight = match issue.severity.as_str() {
                "high" => 0.15,
                "medium" => 0.08,
                _ => 0.03,
            };
            penalty += (issue.count as f64 / total as f64) * weight;
        }

        (1.0 - penalty).max(0.0)
    }

    fn compute_text_length_stats(texts: &[String]) -> TextLengthStats {
        if texts.is_empty() {
            return TextLengthStats {
                min_chars: 0, max_chars: 0, avg_chars: 0.0, median_chars: 0.0,
                min_words: 0, max_words: 0, avg_words: 0.0, median_words: 0.0,
                length_buckets: Vec::new(),
            };
        }

        let char_lens: Vec<usize> = texts.iter().map(|t| t.chars().count()).collect();
        let word_lens: Vec<usize> = texts.iter()
            .map(|t| t.split_whitespace().count())
            .collect();

        let mut sorted_c = char_lens.clone();
        sorted_c.sort();
        let mut sorted_w = word_lens.clone();
        sorted_w.sort();

        let n = texts.len();

        let buckets = vec![
            LengthBucket {
                label: "极短 (< 10字)".to_string(),
                min_chars: 0,
                max_chars: Some(10),
                count: char_lens.iter().filter(|&&c| c < 10).count(),
                ratio: char_lens.iter().filter(|&&c| c < 10).count() as f64 / n as f64,
            },
            LengthBucket {
                label: "短 (10-100字)".to_string(),
                min_chars: 10,
                max_chars: Some(100),
                count: char_lens.iter().filter(|&&c| c >= 10 && c < 100).count(),
                ratio: char_lens.iter().filter(|&&c| c >= 10 && c < 100).count() as f64 / n as f64,
            },
            LengthBucket {
                label: "中等 (100-500字)".to_string(),
                min_chars: 100,
                max_chars: Some(500),
                count: char_lens.iter().filter(|&&c| c >= 100 && c < 500).count(),
                ratio: char_lens.iter().filter(|&&c| c >= 100 && c < 500).count() as f64 / n as f64,
            },
            LengthBucket {
                label: "长 (500-2000字)".to_string(),
                min_chars: 500,
                max_chars: Some(2000),
                count: char_lens.iter().filter(|&&c| c >= 500 && c < 2000).count(),
                ratio: char_lens.iter().filter(|&&c| c >= 500 && c < 2000).count() as f64 / n as f64,
            },
            LengthBucket {
                label: "超长 (> 2000字)".to_string(),
                min_chars: 2000,
                max_chars: None,
                count: char_lens.iter().filter(|&&c| c >= 2000).count(),
                ratio: char_lens.iter().filter(|&&c| c >= 2000).count() as f64 / n as f64,
            },
        ];

        TextLengthStats {
            min_chars: *char_lens.iter().min().unwrap_or(&0),
            max_chars: *char_lens.iter().max().unwrap_or(&0),
            avg_chars: char_lens.iter().sum::<usize>() as f64 / n as f64,
            median_chars: sorted_c[n / 2] as f64,
            min_words: *word_lens.iter().min().unwrap_or(&0),
            max_words: *word_lens.iter().max().unwrap_or(&0),
            avg_words: word_lens.iter().sum::<usize>() as f64 / n as f64,
            median_words: sorted_w[n / 2] as f64,
            length_buckets: buckets,
        }
    }

    fn compute_token_stats(texts: &[String]) -> TokenStats {
        let n = texts.len();
        if n == 0 {
            return TokenStats {
                total_tokens: 0, unique_tokens: 0,
                avg_tokens_per_sample: 0.0,
                token_length_distribution: Vec::new(),
            };
        }

        let mut all_tokens = Vec::new();
        let mut token_counts: Vec<usize> = Vec::new();

        for text in texts {
            let tokens: Vec<&str> = text.split_whitespace().collect();
            token_counts.push(tokens.len());
            all_tokens.extend(tokens);
        }

        let mut unique: HashMap<&str, usize> = HashMap::new();
        for token in &all_tokens {
            *unique.entry(token).or_insert(0) += 1;
        }

        let mut sorted_tc = token_counts.clone();
        sorted_tc.sort();

        let buckets = vec![
            TokenLengthBucket {
                label: "< 10 tokens".to_string(),
                min_tokens: 0,
                max_tokens: Some(10),
                count: token_counts.iter().filter(|&&c| c < 10).count(),
                ratio: token_counts.iter().filter(|&&c| c < 10).count() as f64 / n as f64,
            },
            TokenLengthBucket {
                label: "10-50 tokens".to_string(),
                min_tokens: 10,
                max_tokens: Some(50),
                count: token_counts.iter().filter(|&&c| c >= 10 && c < 50).count(),
                ratio: token_counts.iter().filter(|&&c| c >= 10 && c < 50).count() as f64 / n as f64,
            },
            TokenLengthBucket {
                label: "50-200 tokens".to_string(),
                min_tokens: 50,
                max_tokens: Some(200),
                count: token_counts.iter().filter(|&&c| c >= 50 && c < 200).count(),
                ratio: token_counts.iter().filter(|&&c| c >= 50 && c < 200).count() as f64 / n as f64,
            },
            TokenLengthBucket {
                label: "200-512 tokens".to_string(),
                min_tokens: 200,
                max_tokens: Some(512),
                count: token_counts.iter().filter(|&&c| c >= 200 && c < 512).count(),
                ratio: token_counts.iter().filter(|&&c| c >= 200 && c < 512).count() as f64 / n as f64,
            },
            TokenLengthBucket {
                label: "> 512 tokens".to_string(),
                min_tokens: 512,
                max_tokens: None,
                count: token_counts.iter().filter(|&&c| c >= 512).count(),
                ratio: token_counts.iter().filter(|&&c| c >= 512).count() as f64 / n as f64,
            },
        ];

        TokenStats {
            total_tokens: all_tokens.len(),
            unique_tokens: unique.len(),
            avg_tokens_per_sample: token_counts.iter().sum::<usize>() as f64 / n as f64,
            token_length_distribution: buckets,
        }
    }

    fn compute_vocabulary_stats(texts: &[String]) -> VocabularyStats {
        let mut all_words = Vec::new();
        for text in texts {
            for word in text.split_whitespace() {
                let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric());
                if !cleaned.is_empty() {
                    all_words.push(cleaned.to_lowercase());
                }
            }
        }

        let total = all_words.len();
        if total == 0 {
            return VocabularyStats {
                total_words: 0, unique_words: 0,
                type_token_ratio: 0.0, hapax_legomena: 0, hapax_ratio: 0.0,
            };
        }

        let mut word_counts: HashMap<String, usize> = HashMap::new();
        for word in &all_words {
            *word_counts.entry(word.clone()).or_insert(0) += 1;
        }

        let unique = word_counts.len();
        let hapax = word_counts.values().filter(|&&c| c == 1).count();

        VocabularyStats {
            total_words: total,
            unique_words: unique,
            type_token_ratio: unique as f64 / total as f64,
            hapax_legomena: hapax,
            hapax_ratio: hapax as f64 / total as f64,
        }
    }

    fn compute_ngram_diversity(texts: &[String]) -> NgramDiversity {
        let mut bigrams = HashMap::new();
        let mut trigrams = HashMap::new();
        let mut total_bigrams = 0usize;
        let mut total_trigrams = 0usize;
        let mut repeated_ngrams = 0usize;

        for text in texts {
            let words: Vec<&str> = text.split_whitespace().collect();
            if words.len() < 2 {
                continue;
            }

            for i in 0..words.len() - 1 {
                let bg = (words[i].to_lowercase(), words[i + 1].to_lowercase());
                *bigrams.entry(bg).or_insert(0) += 1;
                total_bigrams += 1;
            }

            for i in 0..words.len().saturating_sub(2) {
                let tg = (
                    words[i].to_lowercase(),
                    words[i + 1].to_lowercase(),
                    words[i + 2].to_lowercase(),
                );
                *trigrams.entry(tg).or_insert(0) += 1;
                total_trigrams += 1;
            }
        }

        for &count in bigrams.values() {
            if count > 3 {
                repeated_ngrams += count - 1;
            }
        }

        let bigram_div = if total_bigrams > 0 {
            bigrams.len() as f64 / total_bigrams as f64
        } else {
            0.0
        };

        let trigram_div = if total_trigrams > 0 {
            trigrams.len() as f64 / total_trigrams as f64
        } else {
            0.0
        };

        let rep_score = if total_bigrams > 0 {
            1.0 - (repeated_ngrams as f64 / total_bigrams as f64).min(1.0)
        } else {
            1.0
        };

        NgramDiversity {
            bigram_diversity: bigram_div,
            trigram_diversity: trigram_div,
            repetition_score: rep_score,
        }
    }

    fn detect_text_quality_issues(
        texts: &[String],
        length_stats: &TextLengthStats,
    ) -> Vec<TextQualityIssue> {
        let n = texts.len();
        if n == 0 {
            return Vec::new();
        }

        let mut issues = Vec::new();

        let empty_count = texts.iter().filter(|t| t.trim().is_empty()).count();
        if empty_count > 0 {
            issues.push(TextQualityIssue {
                issue_type: "empty_texts".to_string(),
                count: empty_count,
                severity: "high".to_string(),
                description: format!("{} 条文本为空", empty_count),
            });
        }

        let very_short = texts.iter()
            .filter(|t| !t.trim().is_empty() && t.chars().count() < 5)
            .count();
        if very_short > 0 && very_short as f64 / n as f64 > 0.05 {
            issues.push(TextQualityIssue {
                issue_type: "very_short_texts".to_string(),
                count: very_short,
                severity: "medium".to_string(),
                description: format!("{} 条文本极短 (< 5字符)，可能信息不足", very_short),
            });
        }

        let special_char_heavy = texts.iter()
            .filter(|t| {
                let special = t.chars().filter(|c| !c.is_alphanumeric() && !c.is_whitespace()).count();
                let total = t.chars().count();
                total > 0 && special as f64 / total as f64 > 0.5
            })
            .count();
        if special_char_heavy > 0 {
            issues.push(TextQualityIssue {
                issue_type: "special_char_heavy".to_string(),
                count: special_char_heavy,
                severity: "low".to_string(),
                description: format!("{} 条文本特殊字符占比 > 50%", special_char_heavy),
            });
        }

        if length_stats.avg_chars > 2000.0 {
            issues.push(TextQualityIssue {
                issue_type: "very_long_texts".to_string(),
                count: texts.iter().filter(|t| t.chars().count() > 2000).count(),
                severity: "medium".to_string(),
                description: "文本平均长度过长，可能需要截断或分段".to_string(),
            });
        }

        issues
    }

    fn compute_text_quality_score(
        issues: &[TextQualityIssue],
        _length_stats: &TextLengthStats,
        vocab_stats: &VocabularyStats,
        total: usize,
    ) -> f64 {
        if total == 0 {
            return 0.0;
        }

        let mut penalty = 0.0;
        for issue in issues {
            let weight = match issue.severity.as_str() {
                "high" => 0.15,
                "medium" => 0.08,
                _ => 0.03,
            };
            penalty += (issue.count as f64 / total as f64) * weight;
        }

        let vocab_bonus = if vocab_stats.type_token_ratio > 0.1 {
            0.05
        } else if vocab_stats.type_token_ratio > 0.05 {
            0.02
        } else {
            0.0
        };

        (1.0 - penalty + vocab_bonus).max(0.0).min(1.0)
    }

    fn generate_image_recommendations(report: &ImageAnalysisReport) -> Vec<String> {
        let mut recs = Vec::new();

        if report.image_quality_score >= 0.9 {
            recs.push("✅ 图像数据集质量优秀".to_string());
        } else if report.image_quality_score >= 0.7 {
            recs.push("🟢 图像数据集质量良好".to_string());
        } else {
            recs.push(format!(
                "🟡 图像数据集质量一般（{:.0}分），建议关注以下问题",
                report.image_quality_score * 100.0
            ));
        }

        for issue in &report.quality_issues {
            recs.push(format!("  - {}: {}", issue.issue_type, issue.description));
        }

        let tiny_ratio = report.resolution_stats.resolution_buckets.first()
            .map(|b| b.ratio).unwrap_or(0.0);
        if tiny_ratio > 0.1 {
            recs.push("💡 建议过滤掉分辨率 < 64x64 的图片".to_string());
        }

        if report.resolution_stats.max_width as f64 / report.resolution_stats.min_width.max(1) as f64 > 10.0 {
            recs.push("💡 分辨率差异过大，建议统一resize到目标尺寸".to_string());
        }

        recs
    }

    fn generate_text_recommendations(report: &TextAnalysisReport) -> Vec<String> {
        let mut recs = Vec::new();

        if report.text_quality_score >= 0.9 {
            recs.push("✅ 文本数据集质量优秀".to_string());
        } else if report.text_quality_score >= 0.7 {
            recs.push("🟢 文本数据集质量良好".to_string());
        } else {
            recs.push(format!(
                "🟡 文本数据集质量一般（{:.0}分），建议关注以下问题",
                report.text_quality_score * 100.0
            ));
        }

        for issue in &report.quality_issues {
            recs.push(format!("  - {}: {}", issue.issue_type, issue.description));
        }

        if report.vocabulary_stats.type_token_ratio < 0.02 {
            recs.push("⚠️ 词汇多样性极低（TTR < 0.02），数据可能存在大量重复".to_string());
        }

        if report.ngram_diversity.repetition_score < 0.5 {
            recs.push("⚠️ N-gram重复度高，建议进行去重处理".to_string());
        }

        if report.length_stats.avg_chars > 2000.0 {
            recs.push(format!(
                "💡 平均文本长度 {:.0} 字符，建议设置max_length截断",
                report.length_stats.avg_chars
            ));
        }

        recs
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub path: String,
    pub width: usize,
    pub height: usize,
    pub format: String,
    pub file_size_bytes: u64,
}
