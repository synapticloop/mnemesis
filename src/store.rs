use crate::model::ProjectContract;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn default_root() -> PathBuf {
        if let Ok(path) = std::env::var("MNEMESIS_HOME") {
            return PathBuf::from(path);
        }
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        home.join(".mnemesis")
    }

    /// Create both the projects/ and drafts/ directories.
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(self.projects_dir())
            .with_context(|| format!("create projects at {}", self.projects_dir().display()))?;
        fs::create_dir_all(self.drafts_dir())
            .with_context(|| format!("create drafts at {}", self.drafts_dir().display()))?;
        Ok(())
    }

    /// All project contracts in the registry, sorted by name.
    pub fn list_projects(&self) -> Result<Vec<ProjectContract>> {
        self.init()?;
        let mut projects = Vec::new();
        for entry in fs::read_dir(self.projects_dir())? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
                continue;
            }
            projects.push(read_contract(&path)?);
        }
        projects.sort_by(|a, b| a.project.name.cmp(&b.project.name));
        Ok(projects)
    }

    /// Load a single project by exact name. Returns None if not present.
    pub fn load_project(&self, name: &str) -> Result<Option<ProjectContract>> {
        let path = self.project_path(name);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(read_contract(&path)?))
    }

    /// Copy a contract from the registry into the drafts area for editing.
    pub fn seed_draft(&self, name: &str, contract: &ProjectContract) -> Result<PathBuf> {
        self.init()?;
        let path = self.draft_path(name);
        let yaml = yaml_serde::to_string(contract)
            .with_context(|| format!("serialize draft for '{}'", name))?;
        let tmp = path.with_extension("yaml.tmp");
        fs::write(&tmp, yaml).with_context(|| format!("write {}", tmp.display()))?;
        fs::rename(&tmp, &path).with_context(|| format!("replace {}", path.display()))?;
        Ok(path)
    }

    /// Read the draft for a project. Returns None if no draft exists.
    pub fn read_draft(&self, name: &str) -> Result<Option<ProjectContract>> {
        let path = self.draft_path(name);
        if !path.exists() {
            return Ok(None);
        }
        let contract = read_contract(&path)?;
        Ok(Some(contract))
    }

    /// Write a contract into the registry, atomically. Validates first.
    pub fn write_project(&self, contract: &ProjectContract) -> Result<PathBuf> {
        contract.validate()?;
        self.init()?;
        let path = self.project_path(&contract.project.name);
        let yaml = yaml_serde::to_string(contract)
            .with_context(|| format!("serialize project '{}'", contract.project.name))?;
        let tmp = path.with_extension("yaml.tmp");
        fs::write(&tmp, yaml).with_context(|| format!("write {}", tmp.display()))?;
        fs::rename(&tmp, &path).with_context(|| format!("replace {}", path.display()))?;
        Ok(path)
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.root.join("projects")
    }

    pub fn drafts_dir(&self) -> PathBuf {
        self.root.join("drafts")
    }

    pub fn project_path(&self, name: &str) -> PathBuf {
        self.projects_dir().join(format!("{name}.yaml"))
    }

    pub fn draft_path(&self, name: &str) -> PathBuf {
        self.drafts_dir().join(format!("{name}.yaml"))
    }
}

fn read_contract(path: &Path) -> Result<ProjectContract> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let contract: ProjectContract =
        yaml_serde::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    contract
        .validate()
        .with_context(|| format!("validate {}", path.display()))?;
    Ok(contract)
}