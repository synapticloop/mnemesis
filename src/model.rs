use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectContract {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub project: Project,
    #[serde(default)]
    pub inputs: Vec<Input>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Input {
    pub name: String,
    pub description: String,
    #[serde(rename = "type", default)]
    pub input_type: Option<InputType>,
    #[serde(default)]
    pub outputs: Vec<Output>,
    #[serde(default)]
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InputType {
    File,
    Directory,
    Text,
    Url,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Output {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub output_type: OutputType,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OutputType {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Action {
    #[serde(rename = "type")]
    pub action_type: String,
    pub instructions: String,
}

fn default_schema_version() -> u32 {
    1
}

impl ProjectContract {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            bail!("unsupported schema_version {}; expected 1", self.schema_version);
        }
        validate_slug("project name", &self.project.name)?;
        validate_nonempty("project description", &self.project.description)?;

        let mut input_names = HashSet::new();
        for input in &self.inputs {
            validate_slug("input name", &input.name)?;
            validate_nonempty("input description", &input.description)?;
            if !input_names.insert(&input.name) {
                bail!("duplicate input name '{}'", input.name);
            }

            let mut output_names = HashSet::new();
            for output in &input.outputs {
                validate_slug("output name", &output.name)?;
                validate_nonempty("output location", &output.location)?;
                if !output_names.insert(&output.name) {
                    bail!("duplicate output name '{}' in input '{}'", output.name, input.name);
                }
            }

            let mut action_types = HashSet::new();
            for action in &input.actions {
                validate_slug("action type", &action.action_type)?;
                validate_nonempty("action instructions", &action.instructions)?;
                if !action_types.insert(&action.action_type) {
                    bail!("duplicate action type '{}' in input '{}'", action.action_type, input.name);
                }
            }
        }
        Ok(())
    }
}

fn validate_nonempty(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label} must not be empty");
    }
    Ok(())
}

fn validate_slug(label: &str, value: &str) -> Result<()> {
    validate_nonempty(label, value)?;
    let valid = value
        .chars()
        .enumerate()
        .all(|(i, c)| c.is_ascii_lowercase() || c.is_ascii_digit() || (c == '-' && i > 0));
    if !valid || value.ends_with('-') {
        bail!("{label} '{value}' must match [a-z0-9][a-z0-9-]*");
    }
    Ok(())
}
