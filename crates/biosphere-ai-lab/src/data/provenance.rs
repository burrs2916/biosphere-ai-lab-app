use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub record_id: String,
    pub dataset_id: String,
    pub row_index: usize,
    pub source: DataSource,
    pub license: LicenseInfo,
    pub collection_info: CollectionInfo,
    pub processing_history: Vec<ProcessingStep>,
    pub usage_restrictions: Vec<UsageRestriction>,
    pub consent_info: Option<ConsentInfo>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub source_type: SourceType,
    pub source_name: String,
    pub source_url: Option<String>,
    pub source_organization: Option<String>,
    pub acquisition_method: AcquisitionMethod,
    pub acquisition_date: Option<String>,
    pub original_format: Option<String>,
    pub raw_data_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    PublicDataset,
    WebCrawl,
    UserGenerated,
    Synthetic,
    Proprietary,
    ThirdParty,
    ResearchInstitution,
    Government,
    Unknown,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PublicDataset => write!(f, "public_dataset"),
            Self::WebCrawl => write!(f, "web_crawl"),
            Self::UserGenerated => write!(f, "user_generated"),
            Self::Synthetic => write!(f, "synthetic"),
            Self::Proprietary => write!(f, "proprietary"),
            Self::ThirdParty => write!(f, "third_party"),
            Self::ResearchInstitution => write!(f, "research_institution"),
            Self::Government => write!(f, "government"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionMethod {
    DirectDownload,
    ApiAccess,
    ManualCollection,
    WebScraping,
    DataAgreement,
    Generated,
    Purchased,
    Unknown,
}

impl std::fmt::Display for AcquisitionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DirectDownload => write!(f, "direct_download"),
            Self::ApiAccess => write!(f, "api_access"),
            Self::ManualCollection => write!(f, "manual_collection"),
            Self::WebScraping => write!(f, "web_scraping"),
            Self::DataAgreement => write!(f, "data_agreement"),
            Self::Generated => write!(f, "generated"),
            Self::Purchased => write!(f, "purchased"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub license_type: LicenseType,
    pub license_name: String,
    pub license_url: Option<String>,
    pub allows_commercial_use: bool,
    pub allows_derivatives: bool,
    pub requires_attribution: bool,
    pub requires_share_alike: bool,
    pub custom_terms: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseType {
    MIT,
    Apache2,
    CC0,
    CCBY,
    CCBYSA,
    CCBYNC,
    CCBYNCSA,
    GPL,
    ODCBY,
    PDDL,
    Custom,
    Unknown,
    Proprietary,
}

impl std::fmt::Display for LicenseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MIT => write!(f, "MIT"),
            Self::Apache2 => write!(f, "Apache-2.0"),
            Self::CC0 => write!(f, "CC0"),
            Self::CCBY => write!(f, "CC-BY"),
            Self::CCBYSA => write!(f, "CC-BY-SA"),
            Self::CCBYNC => write!(f, "CC-BY-NC"),
            Self::CCBYNCSA => write!(f, "CC-BY-NC-SA"),
            Self::GPL => write!(f, "GPL"),
            Self::ODCBY => write!(f, "ODC-BY"),
            Self::PDDL => write!(f, "PDDL"),
            Self::Custom => write!(f, "custom"),
            Self::Unknown => write!(f, "unknown"),
            Self::Proprietary => write!(f, "proprietary"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfo {
    pub collection_method: String,
    pub collection_period: Option<CollectionPeriod>,
    pub geographic_scope: Option<Vec<String>>,
    pub language: Option<String>,
    pub demographic_info: Option<HashMap<String, String>>,
    pub quality_checks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionPeriod {
    pub start_date: String,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStep {
    pub step_name: String,
    pub step_description: String,
    pub tool_used: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub input_hash: Option<String>,
    pub output_hash: Option<String>,
    pub performed_at: String,
    pub performed_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRestriction {
    pub restriction_type: RestrictionType,
    pub description: String,
    pub applies_to: Vec<String>,
    pub effective_until: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestrictionType {
    NoCommercialUse,
    NoDerivatives,
    AttributionRequired,
    ShareAlikeRequired,
    GeographicRestriction,
    TemporalRestriction,
    PurposeRestriction,
    DataSubjectRights,
    ExportControl,
    Confidentiality,
}

impl std::fmt::Display for RestrictionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoCommercialUse => write!(f, "no_commercial_use"),
            Self::NoDerivatives => write!(f, "no_derivatives"),
            Self::AttributionRequired => write!(f, "attribution_required"),
            Self::ShareAlikeRequired => write!(f, "share_alike_required"),
            Self::GeographicRestriction => write!(f, "geographic_restriction"),
            Self::TemporalRestriction => write!(f, "temporal_restriction"),
            Self::PurposeRestriction => write!(f, "purpose_restriction"),
            Self::DataSubjectRights => write!(f, "data_subject_rights"),
            Self::ExportControl => write!(f, "export_control"),
            Self::Confidentiality => write!(f, "confidentiality"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentInfo {
    pub consent_type: ConsentType,
    pub consent_date: Option<String>,
    pub consent_scope: String,
    pub withdrawal_possible: bool,
    pub withdrawal_method: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentType {
    Explicit,
    Implicit,
    OptIn,
    OptOut,
    NotApplicable,
    Unknown,
}

impl std::fmt::Display for ConsentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Explicit => write!(f, "explicit"),
            Self::Implicit => write!(f, "implicit"),
            Self::OptIn => write!(f, "opt_in"),
            Self::OptOut => write!(f, "opt_out"),
            Self::NotApplicable => write!(f, "not_applicable"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceReport {
    pub dataset_id: String,
    pub total_records: usize,
    pub records_with_provenance: usize,
    pub source_distribution: HashMap<String, usize>,
    pub license_distribution: HashMap<String, usize>,
    pub restriction_summary: Vec<RestrictionSummary>,
    pub compliance_issues: Vec<ComplianceIssue>,
    pub overall_compliance_score: f64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestrictionSummary {
    pub restriction_type: RestrictionType,
    pub affected_records: usize,
    pub affected_ratio: f64,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceIssue {
    pub issue_type: String,
    pub description: String,
    pub severity: String,
    pub affected_records: usize,
    pub recommendation: String,
}

pub struct ProvenanceTracker {
    records: Vec<ProvenanceRecord>,
}

impl ProvenanceTracker {
    pub fn new() -> Self {
        Self { records: Vec::new() }
    }

    pub fn add_record(&mut self, record: ProvenanceRecord) {
        self.records.push(record);
    }

    pub fn get_record(&self, row_index: usize) -> Option<&ProvenanceRecord> {
        self.records.iter().find(|r| r.row_index == row_index)
    }

    pub fn generate_report(&self, dataset_id: &str) -> ProvenanceReport {
        let total = self.records.len();
        let with_prov = self.records.iter()
            .filter(|r| r.source.source_type != SourceType::Unknown)
            .count();

        let mut source_dist = HashMap::new();
        let mut license_dist = HashMap::new();

        for record in &self.records {
            *source_dist.entry(record.source.source_type.to_string()).or_insert(0) += 1;
            *license_dist.entry(record.license.license_type.to_string()).or_insert(0) += 1;
        }

        let mut restriction_summary = Vec::new();
        let mut restriction_counts: HashMap<RestrictionType, usize> = HashMap::new();

        for record in &self.records {
            for restriction in &record.usage_restrictions {
                *restriction_counts.entry(restriction.restriction_type).or_insert(0) += 1;
            }
        }

        for (rt, count) in &restriction_counts {
            restriction_summary.push(RestrictionSummary {
                restriction_type: *rt,
                affected_records: *count,
                affected_ratio: if total > 0 { *count as f64 / total as f64 } else { 0.0 },
                severity: match rt {
                    RestrictionType::NoCommercialUse | RestrictionType::ExportControl => "high".to_string(),
                    RestrictionType::Confidentiality | RestrictionType::DataSubjectRights => "critical".to_string(),
                    _ => "medium".to_string(),
                },
            });
        }

        let mut compliance_issues = Vec::new();
        let mut compliance_score: f64 = 1.0;

        let unknown_license = self.records.iter()
            .filter(|r| r.license.license_type == LicenseType::Unknown)
            .count();
        if unknown_license > 0 {
            compliance_issues.push(ComplianceIssue {
                issue_type: "unknown_license".to_string(),
                description: format!("{} records have unknown license", unknown_license),
                severity: "high".to_string(),
                affected_records: unknown_license,
                recommendation: "标注所有数据的许可证信息".to_string(),
            });
            compliance_score -= 0.2;
        }

        let no_consent = self.records.iter()
            .filter(|r| r.consent_info.is_none() && r.source.source_type == SourceType::UserGenerated)
            .count();
        if no_consent > 0 {
            compliance_issues.push(ComplianceIssue {
                issue_type: "missing_consent".to_string(),
                description: format!("{} user-generated records lack consent info", no_consent),
                severity: "critical".to_string(),
                affected_records: no_consent,
                recommendation: "收集或记录用户数据的使用同意".to_string(),
            });
            compliance_score -= 0.3;
        }

        let commercial_restricted = self.records.iter()
            .filter(|r| {
                r.usage_restrictions.iter().any(|u| u.restriction_type == RestrictionType::NoCommercialUse)
            })
            .count();
        if commercial_restricted > 0 {
            compliance_issues.push(ComplianceIssue {
                issue_type: "commercial_restriction".to_string(),
                description: format!("{} records have commercial use restrictions", commercial_restricted),
                severity: "medium".to_string(),
                affected_records: commercial_restricted,
                recommendation: "商业用途前确认数据许可".to_string(),
            });
        }

        compliance_score = compliance_score.max(0.0);

        let mut recommendations = Vec::new();
        if unknown_license > 0 {
            recommendations.push("🔴 为所有数据标注许可证信息".to_string());
        }
        if no_consent > 0 {
            recommendations.push("🔴 用户生成数据需要记录同意信息".to_string());
        }
        if commercial_restricted > 0 {
            recommendations.push("⚠️ 商业用途前检查数据使用限制".to_string());
        }
        if compliance_score >= 0.9 {
            recommendations.push("✅ 数据合规性良好".to_string());
        }

        ProvenanceReport {
            dataset_id: dataset_id.to_string(),
            total_records: total,
            records_with_provenance: with_prov,
            source_distribution: source_dist,
            license_distribution: license_dist,
            restriction_summary,
            compliance_issues,
            overall_compliance_score: compliance_score,
            recommendations,
        }
    }

    pub fn check_commercial_use(&self) -> Result<bool, Vec<String>> {
        let mut issues = Vec::new();

        for record in &self.records {
            if !record.license.allows_commercial_use {
                issues.push(format!(
                    "Row {}: license '{}' does not allow commercial use",
                    record.row_index, record.license.license_name
                ));
            }
            for restriction in &record.usage_restrictions {
                if restriction.restriction_type == RestrictionType::NoCommercialUse {
                    issues.push(format!(
                        "Row {}: has commercial use restriction: {}",
                        record.row_index, restriction.description
                    ));
                }
            }
        }

        if issues.is_empty() {
            Ok(true)
        } else {
            Err(issues)
        }
    }

    pub fn filter_by_license(&self, allowed_licenses: &[LicenseType]) -> Vec<usize> {
        self.records.iter()
            .filter(|r| allowed_licenses.contains(&r.license.license_type))
            .map(|r| r.row_index)
            .collect()
    }

    pub fn get_attribution_text(&self) -> String {
        let mut attributions = std::collections::HashSet::new();
        for record in &self.records {
            if record.license.requires_attribution {
                attributions.insert(format!(
                    "{} (via {})",
                    record.source.source_name,
                    record.source.source_type
                ));
            }
        }
        if attributions.is_empty() {
            "No attribution required".to_string()
        } else {
            let mut parts: Vec<String> = attributions.into_iter().collect();
            parts.sort();
            parts.join("; ")
        }
    }
}

impl Default for ProvenanceTracker {
    fn default() -> Self {
        Self::new()
    }
}
