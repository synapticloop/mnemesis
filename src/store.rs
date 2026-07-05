use crate::model::ProjectContract;
use anyhow::{anyhow, Context, Result};
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
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        home.join(".mnemesis")
    }

    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(self.projects_dir())
            .with_context(|| format!("create store at {}", self.root.display()))
    }

    pub fn list(&self) -> Result<Vec<ProjectContract>> {
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

    pub fn load(&self, name: &str) -> Result<ProjectContract> {
        let path = self.project_path(name);
        if !path.exists() {
            return Err(anyhow!("project '{}' not found", name));
        }
        read_contract(&path)
    }

    pub fn write(&self, contract: &ProjectContract, overwrite: bool) -> Result<PathBuf> {
        contract.validate()?;
        self.init()?;
        let path = self.project_path(&contract.project.name);
        if path.exists() && !overwrite {
            return Err(anyhow!("project '{}' already exists", contract.project.name));
        }

        let yaml = yaml_serde::to_string(contract)?;
        let tmp = path.with_extension("yaml.tmp");
        fs::write(&tmp, yaml).with_context(|| format!("write {}", tmp.display()))?;
        fs::rename(&tmp, &path).with_context(|| format!("replace {}", path.display()))?;
        Ok(path)
    }

    pub fn remove(&self, name: &str) -> Result<()> {
        let path = self.project_path(name);
        if !path.exists() {
            return Err(anyhow!("project '{}' not found", name));
        }
        fs::remove_file(path)?;
        Ok(())
    }

    fn projects_dir(&self) -> PathBuf {
        self.root.join("projects")
    }

    fn project_path(&self, name: &str) -> PathBuf {
        self.projects_dir().join(format!("{name}.yaml"))
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
