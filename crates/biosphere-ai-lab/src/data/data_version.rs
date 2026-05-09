use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataVersionConfig {
    pub repo_path: PathBuf,
    pub compression: bool,
    pub chunk_size: usize,
    pub max_versions: Option<usize>,
}

impl Default for DataVersionConfig {
    fn default() -> Self {
        Self {
            repo_path: PathBuf::from(".biosphere_data"),
            compression: false,
            chunk_size: 64 * 1024,
            max_versions: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentHash(String);

impl ContentHash {
    pub fn new(hash: &str) -> Self {
        Self(hash.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0[..8.min(self.0.len())])
    }
}

impl std::str::FromStr for ContentHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() >= 8 && s.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(Self(s.to_string()))
        } else {
            Err("Invalid content hash".to_string())
        }
    }
}

fn compute_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn compute_file_hash(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|e| format!("Failed to open file for hashing: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataVersion {
    pub hash: ContentHash,
    pub parent_hash: Option<ContentHash>,
    pub timestamp: u64,
    pub message: String,
    pub author: String,
    pub dataset_name: String,
    pub num_files: usize,
    pub total_size_bytes: u64,
    pub num_rows: Option<usize>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub file_hashes: HashMap<String, String>,
}

impl DataVersion {
    pub fn short_hash(&self) -> String {
        self.hash.to_string()
    }

    pub fn age_seconds(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.timestamp)
    }

    pub fn age_display(&self) -> String {
        let secs = self.age_seconds();
        if secs < 60 {
            format!("{}s ago", secs)
        } else if secs < 3600 {
            format!("{}m ago", secs / 60)
        } else if secs < 86400 {
            format!("{}h ago", secs / 3600)
        } else {
            format!("{}d ago", secs / 86400)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiff {
    pub from_hash: ContentHash,
    pub to_hash: ContentHash,
    pub files_added: Vec<String>,
    pub files_removed: Vec<String>,
    pub files_modified: Vec<String>,
    pub files_unchanged: Vec<String>,
    pub size_change_bytes: i64,
    pub row_change: Option<i64>,
    pub row_diffs: Option<Vec<RowChange>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowChange {
    pub row_index: usize,
    pub change_type: RowChangeType,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RowChangeType {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub head_hash: ContentHash,
    pub created_at: u64,
    pub description: Option<String>,
}

pub struct DataVersionRepo {
    config: DataVersionConfig,
    versions: Vec<DataVersion>,
    branches: HashMap<String, Branch>,
    current_branch: String,
    head_hash: Option<ContentHash>,
}

impl DataVersionRepo {
    pub fn init(config: DataVersionConfig) -> Result<Self, String> {
        let repo_path = &config.repo_path;
        fs::create_dir_all(repo_path)
            .map_err(|e| format!("Failed to create repo dir: {}", e))?;
        fs::create_dir_all(repo_path.join("objects"))
            .map_err(|e| format!("Failed to create objects dir: {}", e))?;
        fs::create_dir_all(repo_path.join("refs"))
            .map_err(|e| format!("Failed to create refs dir: {}", e))?;

        let mut repo = Self {
            config,
            versions: Vec::new(),
            branches: HashMap::new(),
            current_branch: "main".to_string(),
            head_hash: None,
        };

        let main_branch = Branch {
            name: "main".to_string(),
            head_hash: ContentHash::new(""),
            created_at: now_timestamp(),
            description: Some("Default branch".to_string()),
        };
        repo.branches.insert("main".to_string(), main_branch);
        repo.save_refs()?;

        Ok(repo)
    }

    pub fn open(config: DataVersionConfig) -> Result<Self, String> {
        let repo_path = &config.repo_path;
        if !repo_path.exists() {
            return Err(format!("Repository not found at {:?}", repo_path));
        }

        let versions = Self::load_versions(repo_path)?;
        let branches = Self::load_branches(repo_path)?;

        let head_path = repo_path.join("HEAD");
        let current_branch = if head_path.exists() {
            fs::read_to_string(&head_path)
                .unwrap_or_else(|_| "main".to_string())
                .trim()
                .to_string()
        } else {
            "main".to_string()
        };

        let head_hash = branches.get(&current_branch)
            .and_then(|b| {
                if b.head_hash.as_str().is_empty() {
                    None
                } else {
                    Some(b.head_hash.clone())
                }
            });

        Ok(Self {
            config,
            versions,
            branches,
            current_branch,
            head_hash,
        })
    }

    fn load_versions(repo_path: &Path) -> Result<Vec<DataVersion>, String> {
        let versions_path = repo_path.join("versions.json");
        if !versions_path.exists() {
            return Ok(Vec::new());
        }
        let data = fs::read_to_string(&versions_path)
            .map_err(|e| format!("Failed to read versions: {}", e))?;
        serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse versions: {}", e))
    }

    fn save_versions(&self) -> Result<(), String> {
        let versions_path = self.config.repo_path.join("versions.json");
        let data = serde_json::to_string_pretty(&self.versions)
            .map_err(|e| format!("Failed to serialize versions: {}", e))?;
        fs::write(&versions_path, data)
            .map_err(|e| format!("Failed to write versions: {}", e))
    }

    fn load_branches(repo_path: &Path) -> Result<HashMap<String, Branch>, String> {
        let branches_path = repo_path.join("branches.json");
        if !branches_path.exists() {
            let mut map = HashMap::new();
            map.insert("main".to_string(), Branch {
                name: "main".to_string(),
                head_hash: ContentHash::new(""),
                created_at: now_timestamp(),
                description: Some("Default branch".to_string()),
            });
            return Ok(map);
        }
        let data = fs::read_to_string(&branches_path)
            .map_err(|e| format!("Failed to read branches: {}", e))?;
        serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse branches: {}", e))
    }

    fn save_refs(&self) -> Result<(), String> {
        let branches_path = self.config.repo_path.join("branches.json");
        let data = serde_json::to_string_pretty(&self.branches)
            .map_err(|e| format!("Failed to serialize branches: {}", e))?;
        fs::write(&branches_path, data)
            .map_err(|e| format!("Failed to write branches: {}", e))?;

        let head_path = self.config.repo_path.join("HEAD");
        fs::write(&head_path, &self.current_branch)
            .map_err(|e| format!("Failed to write HEAD: {}", e))
    }

    pub fn commit(
        &mut self,
        dataset_name: &str,
        files: &[PathBuf],
        message: &str,
        author: &str,
        tags: Vec<String>,
    ) -> Result<ContentHash, String> {
        let mut file_hashes = HashMap::new();
        let mut total_size = 0u64;
        let mut num_files = 0usize;

        for file_path in files {
            if !file_path.exists() {
                return Err(format!("File not found: {:?}", file_path));
            }

            let hash = compute_file_hash(file_path)?;
            let metadata = fs::metadata(file_path)
                .map_err(|e| format!("Failed to read metadata: {}", e))?;
            total_size += metadata.len();
            num_files += 1;

            let file_name = file_path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            file_hashes.insert(file_name, hash.clone());

            let object_path = self.config.repo_path
                .join("objects")
                .join(&hash);
            if !object_path.exists() {
                fs::copy(file_path, &object_path)
                    .map_err(|e| format!("Failed to store object: {}", e))?;
            }
        }

        let version_data = serde_json::json!({
            "dataset_name": dataset_name,
            "files": file_hashes.keys().collect::<Vec<_>>(),
            "timestamp": now_timestamp(),
            "message": message,
            "author": author,
            "parent": self.head_hash.as_ref().map(|h| h.as_str()),
        });

        let version_json = serde_json::to_string(&version_data)
            .map_err(|e| format!("Failed to serialize version: {}", e))?;
        let version_hash = compute_hash(version_json.as_bytes());

        let version = DataVersion {
            hash: ContentHash::new(&version_hash),
            parent_hash: self.head_hash.clone(),
            timestamp: now_timestamp(),
            message: message.to_string(),
            author: author.to_string(),
            dataset_name: dataset_name.to_string(),
            num_files,
            total_size_bytes: total_size,
            num_rows: None,
            tags,
            metadata: HashMap::new(),
            file_hashes,
        };

        self.versions.push(version);
        self.head_hash = Some(ContentHash::new(&version_hash));

        if let Some(branch) = self.branches.get_mut(&self.current_branch) {
            branch.head_hash = ContentHash::new(&version_hash);
        }

        self.save_versions()?;
        self.save_refs()?;

        if let Some(max) = self.config.max_versions {
            while self.versions.len() > max {
                self.versions.remove(0);
            }
        }

        Ok(ContentHash::new(&version_hash))
    }

    pub fn checkout(&mut self, hash: &ContentHash) -> Result<(), String> {
        let version = self.versions.iter()
            .find(|v| v.hash.as_str() == hash.as_str())
            .ok_or_else(|| format!("Version not found: {}", hash))?;

        self.head_hash = Some(version.hash.clone());

        if let Some(branch) = self.branches.get_mut(&self.current_branch) {
            branch.head_hash = version.hash.clone();
        }

        self.save_refs()
    }

    pub fn checkout_branch(&mut self, branch_name: &str) -> Result<(), String> {
        if !self.branches.contains_key(branch_name) {
            return Err(format!("Branch not found: {}", branch_name));
        }

        self.current_branch = branch_name.to_string();
        self.head_hash = self.branches.get(branch_name)
            .and_then(|b| {
                if b.head_hash.as_str().is_empty() {
                    None
                } else {
                    Some(b.head_hash.clone())
                }
            });

        self.save_refs()
    }

    pub fn create_branch(&mut self, name: &str, description: Option<&str>) -> Result<(), String> {
        if self.branches.contains_key(name) {
            return Err(format!("Branch already exists: {}", name));
        }

        let head = self.head_hash.clone().unwrap_or(ContentHash::new(""));
        self.branches.insert(name.to_string(), Branch {
            name: name.to_string(),
            head_hash: head,
            created_at: now_timestamp(),
            description: description.map(|s| s.to_string()),
        });

        self.save_refs()
    }

    pub fn delete_branch(&mut self, name: &str) -> Result<(), String> {
        if name == "main" {
            return Err("Cannot delete main branch".to_string());
        }
        if self.current_branch == name {
            return Err("Cannot delete current branch".to_string());
        }
        self.branches.remove(name);
        self.save_refs()
    }

    pub fn diff(&self, from: &ContentHash, to: &ContentHash) -> Result<VersionDiff, String> {
        let from_version = self.versions.iter()
            .find(|v| v.hash.as_str() == from.as_str())
            .ok_or_else(|| format!("Version not found: {}", from))?;

        let to_version = self.versions.iter()
            .find(|v| v.hash.as_str() == to.as_str())
            .ok_or_else(|| format!("Version not found: {}", to))?;

        let from_files: HashSet<&String> = from_version.file_hashes.keys().collect();
        let to_files: HashSet<&String> = to_version.file_hashes.keys().collect();

        let files_added: Vec<String> = to_files.difference(&from_files)
            .map(|s| s.to_string())
            .collect();

        let files_removed: Vec<String> = from_files.difference(&to_files)
            .map(|s| s.to_string())
            .collect();

        let mut files_modified = Vec::new();
        let mut files_unchanged = Vec::new();

        for file_name in from_files.intersection(&to_files) {
            let from_hash = from_version.file_hashes.get(*file_name);
            let to_hash = to_version.file_hashes.get(*file_name);
            if from_hash != to_hash {
                files_modified.push(file_name.to_string());
            } else {
                files_unchanged.push(file_name.to_string());
            }
        }

        let size_change = to_version.total_size_bytes as i64 - from_version.total_size_bytes as i64;
        let row_change = match (from_version.num_rows, to_version.num_rows) {
            (Some(f), Some(t)) => Some(t as i64 - f as i64),
            _ => None,
        };

        Ok(VersionDiff {
            from_hash: from.clone(),
            to_hash: to.clone(),
            files_added,
            files_removed,
            files_modified,
            files_unchanged,
            size_change_bytes: size_change,
            row_change,
            row_diffs: None,
        })
    }

    pub fn log(&self, max_count: Option<usize>) -> Vec<&DataVersion> {
        let count = max_count.unwrap_or(self.versions.len());
        self.versions.iter()
            .rev()
            .take(count)
            .collect()
    }

    pub fn get_version(&self, hash: &ContentHash) -> Option<&DataVersion> {
        self.versions.iter().find(|v| v.hash.as_str() == hash.as_str())
    }

    pub fn current_version(&self) -> Option<&DataVersion> {
        self.head_hash.as_ref()
            .and_then(|h| self.get_version(h))
    }

    pub fn current_branch(&self) -> &str {
        &self.current_branch
    }

    pub fn branches(&self) -> Vec<&Branch> {
        self.branches.values().collect()
    }

    pub fn version_count(&self) -> usize {
        self.versions.len()
    }

    pub fn restore_file(&self, hash: &ContentHash, file_name: &str, output_path: &Path) -> Result<(), String> {
        let version = self.get_version(hash)
            .ok_or_else(|| format!("Version not found: {}", hash))?;

        let file_hash = version.file_hashes.get(file_name)
            .ok_or_else(|| format!("File not found in version: {}", file_name))?;

        let object_path = self.config.repo_path
            .join("objects")
            .join(file_hash);

        if !object_path.exists() {
            return Err(format!("Object not found: {}", file_hash));
        }

        fs::copy(&object_path, output_path)
            .map_err(|e| format!("Failed to restore file: {}", e))?;

        Ok(())
    }

    pub fn ancestry(&self, hash: &ContentHash) -> Vec<ContentHash> {
        let mut chain = Vec::new();
        let mut current = Some(hash.clone());

        while let Some(h) = current {
            chain.push(h.clone());
            if let Some(version) = self.get_version(&h) {
                current = version.parent_hash.clone();
            } else {
                break;
            }
        }

        chain
    }

    pub fn tag_version(&mut self, hash: &ContentHash, tag: &str) -> Result<(), String> {
        if let Some(version) = self.versions.iter_mut().find(|v| v.hash.as_str() == hash.as_str()) {
            if !version.tags.contains(&tag.to_string()) {
                version.tags.push(tag.to_string());
            }
            self.save_versions()?;
            Ok(())
        } else {
            Err(format!("Version not found: {}", hash))
        }
    }
}

fn now_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_repo() -> (DataVersionRepo, PathBuf) {
        let tmp = std::env::temp_dir().join(format!("biosphere_test_repo_{}", rand::random::<u32>()));
        let config = DataVersionConfig {
            repo_path: tmp.clone(),
            ..Default::default()
        };
        let repo = DataVersionRepo::init(config).unwrap();
        (repo, tmp)
    }

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_repo_init() {
        let (repo, tmp) = create_test_repo();
        assert_eq!(repo.version_count(), 0);
        assert_eq!(repo.current_branch(), "main");
        assert!(repo.current_version().is_none());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_commit_and_log() {
        let (mut repo, tmp) = create_test_repo();
        let file = create_test_file(&tmp, "data.csv", "a,b,c\n1,2,3\n");

        let hash = repo.commit(
            "test_dataset",
            &[file],
            "Initial commit",
            "test_user",
            vec![],
        ).unwrap();

        assert_eq!(repo.version_count(), 1);
        assert!(repo.current_version().is_some());

        let log = repo.log(None);
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].hash.as_str(), hash.as_str());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_multiple_commits() {
        let (mut repo, tmp) = create_test_repo();

        let file1 = create_test_file(&tmp, "v1.csv", "a\n1\n");
        let hash1 = repo.commit("ds", &[file1], "v1", "user", vec![]).unwrap();

        let file2 = create_test_file(&tmp, "v2.csv", "a\n1\n2\n");
        let hash2 = repo.commit("ds", &[file2], "v2", "user", vec![]).unwrap();

        assert_eq!(repo.version_count(), 2);

        let ancestry = repo.ancestry(&hash2);
        assert_eq!(ancestry.len(), 2);
        assert_eq!(ancestry[0].as_str(), hash2.as_str());
        assert_eq!(ancestry[1].as_str(), hash1.as_str());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_checkout() {
        let (mut repo, tmp) = create_test_repo();

        let file1 = create_test_file(&tmp, "v1.csv", "a\n1\n");
        let hash1 = repo.commit("ds", &[file1], "v1", "user", vec![]).unwrap();

        let file2 = create_test_file(&tmp, "v2.csv", "a\n1\n2\n");
        let _hash2 = repo.commit("ds", &[file2], "v2", "user", vec![]).unwrap();

        repo.checkout(&hash1).unwrap();
        let current = repo.current_version().unwrap();
        assert_eq!(current.hash.as_str(), hash1.as_str());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_diff() {
        let (mut repo, tmp) = create_test_repo();

        let file1 = create_test_file(&tmp, "v1.csv", "a\n1\n");
        let hash1 = repo.commit("ds", &[file1], "v1", "user", vec![]).unwrap();

        let file2 = create_test_file(&tmp, "v2.csv", "a\n1\n2\n");
        let hash2 = repo.commit("ds", &[file2.clone()], "v2", "user", vec![]).unwrap();

        let diff = repo.diff(&hash1, &hash2).unwrap();
        assert_eq!(diff.from_hash.as_str(), hash1.as_str());
        assert_eq!(diff.to_hash.as_str(), hash2.as_str());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_branch_operations() {
        let (mut repo, tmp) = create_test_repo();

        let file = create_test_file(&tmp, "data.csv", "a\n1\n");
        let _hash = repo.commit("ds", &[file], "initial", "user", vec![]).unwrap();

        repo.create_branch("experiment", Some("test branch")).unwrap();
        assert!(repo.branches().iter().any(|b| b.name == "experiment"));

        repo.checkout_branch("experiment").unwrap();
        assert_eq!(repo.current_branch(), "experiment");

        repo.checkout_branch("main").unwrap();
        repo.delete_branch("experiment").unwrap();
        assert!(!repo.branches().iter().any(|b| b.name == "experiment"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_tag_version() {
        let (mut repo, tmp) = create_test_repo();

        let file = create_test_file(&tmp, "data.csv", "a\n1\n");
        let hash = repo.commit("ds", &[file], "initial", "user", vec![]).unwrap();

        repo.tag_version(&hash, "v1.0").unwrap();

        let version = repo.get_version(&hash).unwrap();
        assert!(version.tags.contains(&"v1.0".to_string()));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_restore_file() {
        let (mut repo, tmp) = create_test_repo();

        let file = create_test_file(&tmp, "data.csv", "col1,col2\n1,2\n");
        let hash = repo.commit("ds", &[file], "initial", "user", vec![]).unwrap();

        let restore_path = tmp.join("restored.csv");
        repo.restore_file(&hash, "data.csv", &restore_path).unwrap();

        assert!(restore_path.exists());
        let content = fs::read_to_string(&restore_path).unwrap();
        assert_eq!(content, "col1,col2\n1,2\n");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_content_hash() {
        let hash = ContentHash::new("abcdef1234567890");
        assert_eq!(hash.to_string(), "abcdef12");
        assert_eq!(hash.as_str(), "abcdef1234567890");
    }
}
