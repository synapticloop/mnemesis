use crate::model::{Action, Input, Output, Project, ProjectContract};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldChange {
    /// A field was added (didn't exist in the old version).
    Added { path: String, value: String },
    /// A field's value differs from the old version.
    Modified { path: String, old: String, new: String },
    /// A field was removed (exists in old, not in new).
    Removed { path: String, old: String },
}

/// Compute field-level changes between two contracts. Each change is identified by
/// a stable path like `project.description`, `inputs[0].outputs[1].location`, or
/// `inputs[2].actions[0].type` so the caller can selectively accept them.
pub fn diff_contracts(old: &ProjectContract, new: &ProjectContract) -> Vec<FieldChange> {
    let mut changes = Vec::new();

    if old.project != new.project {
        diff_project(&old.project, &new.project, &mut changes);
    }
    diff_inputs(&old.inputs, &new.inputs, &mut changes);

    changes
}

fn diff_project(old: &Project, new: &Project, changes: &mut Vec<FieldChange>) {
    if old.name != new.name {
        changes.push(FieldChange::Modified {
            path: "project.name".to_string(),
            old: old.name.clone(),
            new: new.name.clone(),
        });
    }
    if old.description != new.description {
        changes.push(FieldChange::Modified {
            path: "project.description".to_string(),
            old: old.description.clone(),
            new: new.description.clone(),
        });
    }
}

fn diff_inputs(old: &[Input], new: &[Input], changes: &mut Vec<FieldChange>) {
    // Match old inputs to new inputs by name. Unmatched inputs are added or removed.
    for new_input in new {
        match old.iter().find(|i| i.name == new_input.name) {
            Some(old_input) => {
                diff_input(old_input, new_input, changes);
            }
            None => {
                changes.push(FieldChange::Added {
                    path: format!("inputs[{}]", new_input.name),
                    value: format!("input with {} outputs and {} actions", new_input.outputs.len(), new_input.actions.len()),
                });
                diff_input_added(&format!("inputs[{}]", new_input.name), new_input, changes);
            }
        }
    }
    for old_input in old {
        if !new.iter().any(|i| i.name == old_input.name) {
            changes.push(FieldChange::Removed {
                path: format!("inputs[{}]", old_input.name),
                old: format!("input with {} outputs and {} actions", old_input.outputs.len(), old_input.actions.len()),
            });
        }
    }
}

fn diff_input(old: &Input, new: &Input, changes: &mut Vec<FieldChange>) {
    let base = format!("inputs[{}]", new.name);
    if old.description != new.description {
        changes.push(FieldChange::Modified {
            path: format!("{base}.description"),
            old: old.description.clone(),
            new: new.description.clone(),
        });
    }
    if old.input_type != new.input_type {
        changes.push(FieldChange::Modified {
            path: format!("{base}.type"),
            old: format!("{:?}", old.input_type),
            new: format!("{:?}", new.input_type),
        });
    }
    diff_outputs(&old.outputs, &new.outputs, &base, changes);
    diff_actions(&old.actions, &new.actions, &base, changes);
}

fn diff_input_added(base: &str, new: &Input, changes: &mut Vec<FieldChange>) {
    if !new.description.is_empty() {
        changes.push(FieldChange::Added {
            path: format!("{base}.description"),
            value: new.description.clone(),
        });
    }
    if let Some(t) = &new.input_type {
        changes.push(FieldChange::Added {
            path: format!("{base}.type"),
            value: format!("{t:?}").to_lowercase(),
        });
    }
    for output in &new.outputs {
        diff_output_added(&format!("{base}.outputs[{}]", output.name), output, changes);
    }
    for action in &new.actions {
        diff_action_added(&format!("{base}.actions[{}]", action.action_type), action, changes);
    }
}

fn diff_outputs(old: &[Output], new: &[Output], base: &str, changes: &mut Vec<FieldChange>) {
    for new_output in new {
        match old.iter().find(|o| o.name == new_output.name) {
            Some(old_output) => {
                diff_output(old_output, new_output, base, changes);
            }
            None => {
                diff_output_added(&format!("{base}.outputs[{}]", new_output.name), new_output, changes);
            }
        }
    }
    for old_output in old {
        if !new.iter().any(|o| o.name == old_output.name) {
            changes.push(FieldChange::Removed {
                path: format!("{base}.outputs[{}]", old_output.name),
                old: format_output(old_output),
            });
        }
    }
}

fn diff_output(old: &Output, new: &Output, base: &str, changes: &mut Vec<FieldChange>) {
    let path = format!("{base}.outputs[{}]", new.name);
    if old.description != new.description {
        changes.push(FieldChange::Modified {
            path: format!("{path}.description"),
            old: old.description.clone().unwrap_or_default(),
            new: new.description.clone().unwrap_or_default(),
        });
    }
    if old.output_type != new.output_type {
        changes.push(FieldChange::Modified {
            path: format!("{path}.type"),
            old: format!("{:?}", old.output_type).to_lowercase(),
            new: format!("{:?}", new.output_type).to_lowercase(),
        });
    }
    if old.location != new.location {
        changes.push(FieldChange::Modified {
            path: format!("{path}.location"),
            old: old.location.clone(),
            new: new.location.clone(),
        });
    }
}

fn diff_output_added(path: &str, new: &Output, changes: &mut Vec<FieldChange>) {
    changes.push(FieldChange::Added {
        path: path.to_string(),
        value: format_output(new),
    });
    if let Some(desc) = &new.description {
        changes.push(FieldChange::Added {
            path: format!("{path}.description"),
            value: desc.clone(),
        });
    }
    changes.push(FieldChange::Added {
        path: format!("{path}.type"),
        value: format!("{:?}", new.output_type).to_lowercase(),
    });
    changes.push(FieldChange::Added {
        path: format!("{path}.location"),
        value: new.location.clone(),
    });
}

fn diff_actions(old: &[Action], new: &[Action], base: &str, changes: &mut Vec<FieldChange>) {
    for new_action in new {
        match old.iter().find(|a| a.action_type == new_action.action_type) {
            Some(old_action) => {
                diff_action(old_action, new_action, base, changes);
            }
            None => {
                diff_action_added(
                    &format!("{base}.actions[{}]", new_action.action_type),
                    new_action,
                    changes,
                );
            }
        }
    }
    for old_action in old {
        if !new.iter().any(|a| a.action_type == old_action.action_type) {
            changes.push(FieldChange::Removed {
                path: format!("{base}.actions[{}]", old_action.action_type),
                old: old_action.instructions.clone(),
            });
        }
    }
}

fn diff_action(old: &Action, new: &Action, base: &str, changes: &mut Vec<FieldChange>) {
    let path = format!("{base}.actions[{}]", new.action_type);
    if old.instructions != new.instructions {
        changes.push(FieldChange::Modified {
            path: format!("{path}.instructions"),
            old: old.instructions.clone(),
            new: new.instructions.clone(),
        });
    }
}

fn diff_action_added(path: &str, new: &Action, changes: &mut Vec<FieldChange>) {
    changes.push(FieldChange::Added {
        path: path.to_string(),
        value: format!("action with {} chars of instructions", new.instructions.len()),
    });
    changes.push(FieldChange::Added {
        path: format!("{path}.instructions"),
        value: new.instructions.clone(),
    });
}

fn format_output(o: &Output) -> String {
    format!(
        "{} output at {}",
        format!("{:?}", o.output_type).to_lowercase(),
        o.location
    )
}