use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDiscoveryIndex {
    pub datasets: Vec<DiscoveredDataset>,
    pub total_count: usize,
    pub facets: SearchFacets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDataset {
    pub id: String,
    pub name: String,
    pub version: String,
    pub format: String,
    pub rows: usize,
    pub columns: usize,
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub tasks: Vec<String>,
    pub license: Option<String>,
    pub language: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub relevance_score: Option<f64>,
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacets {
    pub formats: Vec<FacetCount>,
    pub tasks: Vec<FacetCount>,
    pub tags: Vec<FacetCount>,
    pub licenses: Vec<FacetCount>,
    pub row_ranges: Vec<FacetCount>,
    pub date_ranges: Vec<FacetCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetCount {
    pub value: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: Option<String>,
    pub formats: Vec<String>,
    pub tasks: Vec<String>,
    pub tags: Vec<String>,
    pub licenses: Vec<String>,
    pub min_rows: Option<usize>,
    pub max_rows: Option<usize>,
    pub has_description: Option<bool>,
    pub sort_by: Option<SortField>,
    pub sort_order: Option<SortOrder>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            text: None,
            formats: Vec::new(),
            tasks: Vec::new(),
            tags: Vec::new(),
            licenses: Vec::new(),
            min_rows: None,
            max_rows: None,
            has_description: None,
            sort_by: None,
            sort_order: None,
            limit: Some(20),
            offset: Some(0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    Relevance,
    Name,
    Rows,
    CreatedAt,
    UpdatedAt,
}

impl std::fmt::Display for SortField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Relevance => write!(f, "relevance"),
            Self::Name => write!(f, "name"),
            Self::Rows => write!(f, "rows"),
            Self::CreatedAt => write!(f, "created_at"),
            Self::UpdatedAt => write!(f, "updated_at"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Asc,
    Desc,
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Asc => write!(f, "asc"),
            Self::Desc => write!(f, "desc"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetUsageStats {
    pub dataset_id: String,
    pub total_experiments: usize,
    pub total_models: usize,
    pub last_used: Option<String>,
    pub most_common_task: Option<String>,
    pub popularity_score: f64,
}

pub struct DataDiscoveryEngine;

impl DataDiscoveryEngine {
    pub fn search(
        datasets: &[DiscoveredDataset],
        query: &SearchQuery,
    ) -> DataDiscoveryIndex {
        let mut results: Vec<&DiscoveredDataset> = datasets.iter().collect();

        if let Some(ref text) = query.text {
            let text_lower = text.to_lowercase();
            results.retain(|d| {
                d.name.to_lowercase().contains(&text_lower)
                    || d.description.as_ref().map(|desc| desc.to_lowercase().contains(&text_lower)).unwrap_or(false)
                    || d.tags.iter().any(|t| t.to_lowercase().contains(&text_lower))
                    || d.tasks.iter().any(|t| t.to_lowercase().contains(&text_lower))
            });
        }

        if !query.formats.is_empty() {
            results.retain(|d| query.formats.iter().any(|f| d.format.eq_ignore_ascii_case(f)));
        }

        if !query.tasks.is_empty() {
            results.retain(|d| {
                query.tasks.iter().any(|t| {
                    d.tasks.iter().any(|dt| dt.eq_ignore_ascii_case(t))
                })
            });
        }

        if !query.tags.is_empty() {
            results.retain(|d| {
                query.tags.iter().any(|t| {
                    d.tags.iter().any(|dt| dt.eq_ignore_ascii_case(t))
                })
            });
        }

        if !query.licenses.is_empty() {
            results.retain(|d| {
                d.license.as_ref().map(|l| {
                    query.licenses.iter().any(|ql| l.eq_ignore_ascii_case(ql))
                }).unwrap_or(false)
            });
        }

        if let Some(min_rows) = query.min_rows {
            results.retain(|d| d.rows >= min_rows);
        }

        if let Some(max_rows) = query.max_rows {
            results.retain(|d| d.rows <= max_rows);
        }

        if let Some(has_desc) = query.has_description {
            results.retain(|d| d.description.is_some() == has_desc);
        }

        let sort_field = query.sort_by.unwrap_or(SortField::Relevance);
        let sort_order = query.sort_order.unwrap_or(SortOrder::Desc);

        match sort_field {
            SortField::Relevance => {
                if query.text.is_some() {
                    results.sort_by(|a, b| {
                        let sa = a.relevance_score.unwrap_or(0.0);
                        let sb = b.relevance_score.unwrap_or(0.0);
                        if sort_order == SortOrder::Desc {
                            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
                        } else {
                            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
                        }
                    });
                }
            }
            SortField::Name => {
                results.sort_by(|a, b| {
                    if sort_order == SortOrder::Desc {
                        b.name.cmp(&a.name)
                    } else {
                        a.name.cmp(&b.name)
                    }
                });
            }
            SortField::Rows => {
                results.sort_by(|a, b| {
                    if sort_order == SortOrder::Desc {
                        b.rows.cmp(&a.rows)
                    } else {
                        a.rows.cmp(&b.rows)
                    }
                });
            }
            SortField::CreatedAt => {
                results.sort_by(|a, b| {
                    if sort_order == SortOrder::Desc {
                        b.created_at.cmp(&a.created_at)
                    } else {
                        a.created_at.cmp(&b.created_at)
                    }
                });
            }
            SortField::UpdatedAt => {
                results.sort_by(|a, b| {
                    if sort_order == SortOrder::Desc {
                        b.updated_at.cmp(&a.updated_at)
                    } else {
                        a.updated_at.cmp(&b.updated_at)
                    }
                });
            }
        }

        let total_count = results.len();

        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(20);
        let paged: Vec<DiscoveredDataset> = results.iter()
            .skip(offset)
            .take(limit)
            .map(|d| {
                let mut ds = (*d).clone();
                if let Some(ref text) = query.text {
                    ds.relevance_score = Some(Self::compute_relevance(d, text));
                    ds.highlights = Self::compute_highlights(d, text);
                }
                ds
            })
            .collect();

        let facets = Self::compute_facets(datasets);

        DataDiscoveryIndex {
            datasets: paged,
            total_count,
            facets,
        }
    }

    fn compute_relevance(dataset: &DiscoveredDataset, query: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let mut score: f64 = 0.0;

        if dataset.name.to_lowercase().contains(&query_lower) {
            score += 0.4;
            if dataset.name.to_lowercase() == query_lower {
                score += 0.3;
            }
        }

        if let Some(ref desc) = dataset.description {
            if desc.to_lowercase().contains(&query_lower) {
                score += 0.2;
            }
        }

        for tag in &dataset.tags {
            if tag.to_lowercase().contains(&query_lower) {
                score += 0.1;
                break;
            }
        }

        for task in &dataset.tasks {
            if task.to_lowercase().contains(&query_lower) {
                score += 0.1;
                break;
            }
        }

        score.min(1.0)
    }

    fn compute_highlights(dataset: &DiscoveredDataset, query: &str) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let mut highlights = Vec::new();

        if dataset.name.to_lowercase().contains(&query_lower) {
            highlights.push(format!("名称匹配: {}", dataset.name));
        }

        if let Some(ref desc) = dataset.description {
            if desc.to_lowercase().contains(&query_lower) {
                let snippet = if desc.len() > 100 {
                    format!("{}...", &desc[..100])
                } else {
                    desc.clone()
                };
                highlights.push(format!("描述匹配: {}", snippet));
            }
        }

        let matching_tags: Vec<&str> = dataset.tags.iter()
            .filter(|t| t.to_lowercase().contains(&query_lower))
            .map(|t| t.as_str())
            .collect();
        if !matching_tags.is_empty() {
            highlights.push(format!("标签匹配: {}", matching_tags.join(", ")));
        }

        highlights
    }

    fn compute_facets(datasets: &[DiscoveredDataset]) -> SearchFacets {
        let mut format_counts: HashMap<&str, usize> = HashMap::new();
        let mut task_counts: HashMap<&str, usize> = HashMap::new();
        let mut tag_counts: HashMap<&str, usize> = HashMap::new();
        let mut license_counts: HashMap<&str, usize> = HashMap::new();
        let mut row_ranges: HashMap<&str, usize> = HashMap::new();
        let mut date_ranges: HashMap<&str, usize> = HashMap::new();

        for ds in datasets {
            *format_counts.entry(ds.format.as_str()).or_insert(0) += 1;

            for task in &ds.tasks {
                *task_counts.entry(task.as_str()).or_insert(0) += 1;
            }

            for tag in &ds.tags {
                *tag_counts.entry(tag.as_str()).or_insert(0) += 1;
            }

            if let Some(ref license) = ds.license {
                *license_counts.entry(license.as_str()).or_insert(0) += 1;
            }

            let row_range = match ds.rows {
                r if r < 1000 => "< 1K",
                r if r < 10000 => "1K-10K",
                r if r < 100000 => "10K-100K",
                r if r < 1000000 => "100K-1M",
                _ => "> 1M",
            };
            *row_ranges.entry(row_range).or_insert(0) += 1;

            let date_range = if ds.created_at.len() >= 7 {
                ds.created_at[..7].to_string()
            } else {
                "unknown".to_string()
            };
            *date_ranges.entry(Box::leak(date_range.into_boxed_str())).or_insert(0) += 1;
        }

        fn to_facet_counts(map: HashMap<&str, usize>) -> Vec<FacetCount> {
            let mut v: Vec<FacetCount> = map.into_iter()
                .map(|(value, count)| FacetCount { value: value.to_string(), count })
                .collect();
            v.sort_by(|a, b| b.count.cmp(&a.count));
            v
        }

        SearchFacets {
            formats: to_facet_counts(format_counts),
            tasks: to_facet_counts(task_counts),
            tags: to_facet_counts(tag_counts),
            licenses: to_facet_counts(license_counts),
            row_ranges: to_facet_counts(row_ranges),
            date_ranges: to_facet_counts(date_ranges),
        }
    }

    pub fn compute_usage_stats(
        dataset_id: &str,
        experiment_count: usize,
        model_count: usize,
        last_used: Option<&str>,
        most_common_task: Option<&str>,
    ) -> DatasetUsageStats {
        let popularity_score = (experiment_count as f64 * 0.6 + model_count as f64 * 0.4)
            .min(100.0) / 100.0;

        DatasetUsageStats {
            dataset_id: dataset_id.to_string(),
            total_experiments: experiment_count,
            total_models: model_count,
            last_used: last_used.map(|s| s.to_string()),
            most_common_task: most_common_task.map(|s| s.to_string()),
            popularity_score,
        }
    }
}
