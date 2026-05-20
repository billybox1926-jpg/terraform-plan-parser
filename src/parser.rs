use glob::Pattern;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ResourceChange {
    pub resource_type: String,
    pub resource_name: String,
    pub action: String,
}

#[derive(Debug, Deserialize)]
pub struct PlanLine {
    pub change: Option<PlanChange>,
}

#[derive(Debug, Deserialize)]
pub struct PlanChange {
    pub resource: Option<PlanResource>,
    pub action: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PlanResource {
    #[serde(default = "unknown_value")]
    pub resource_type: String,
    #[serde(default = "unknown_value")]
    pub resource_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ShowPlan {
    pub resource_changes: Option<Vec<ShowResourceChange>>,
}

#[derive(Debug, Deserialize)]
pub struct ShowResourceChange {
    #[serde(default = "unknown_value", rename = "type")]
    pub resource_type: String,
    #[serde(default = "unknown_value")]
    pub name: String,
    pub change: ShowChange,
}

#[derive(Debug, Deserialize)]
pub struct ShowChange {
    #[serde(default)]
    pub actions: Vec<String>,
}

#[derive(Debug)]
pub enum TerraformInput {
    StdinJson(String),
    Directory(std::path::PathBuf),
    JsonPlanFile(std::path::PathBuf),
    BinaryPlanFile(std::path::PathBuf),
}

impl TerraformInput {
    pub fn requires_terraform(&self) -> bool {
        matches!(self, Self::Directory(_) | Self::BinaryPlanFile(_))
    }
}

#[derive(clap::ValueEnum, Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    Text,
    Json,
    Csv,
    Table,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ChangeCounts {
    pub create: usize,
    pub update: usize,
    pub delete: usize,
}

pub fn unknown_value() -> String {
    "unknown".to_string()
}

pub fn parse_plan_line(line: &str) -> Option<ResourceChange> {
    let line = match serde_json::from_str::<PlanLine>(line) {
        Ok(line) => line,
        Err(error) => {
            tracing::warn!(%error, line, "Skipping invalid Terraform JSON line");
            return None;
        }
    };
    let change = line.change?;
    let resource = change.resource?;

    Some(ResourceChange {
        resource_type: resource.resource_type,
        resource_name: resource.resource_name,
        action: change.action.unwrap_or_else(|| "noop".to_string()),
    })
}

pub fn parse_plan_output(stdout: &str) -> Vec<ResourceChange> {
    if stdout.trim_start().starts_with('{') && stdout.contains("\"resource_changes\"") {
        if let Ok(show_changes) = parse_show_plan_output(stdout) {
            return show_changes;
        }
    }

    stdout.lines().filter_map(parse_plan_line).collect()
}

pub fn parse_show_plan_output(stdout: &str) -> Result<Vec<ResourceChange>, serde_json::Error> {
    let plan = serde_json::from_str::<ShowPlan>(stdout)?;
    Ok(plan
        .resource_changes
        .unwrap_or_default()
        .into_iter()
        .filter_map(|change| {
            let action = action_from_show_actions(&change.change.actions)?;
            Some(ResourceChange {
                resource_type: change.resource_type,
                resource_name: change.name,
                action,
            })
        })
        .collect())
}

pub fn action_from_show_actions(actions: &[String]) -> Option<String> {
    match actions {
        [] => None,
        [action] if action == "no-op" => None,
        [action] => Some(action.clone()),
        [first, second] if first == "delete" && second == "create" => Some("replace".to_string()),
        _ => Some(actions.join("/")),
    }
}

pub fn filter_changes(
    resource_changes: Vec<ResourceChange>,
    settings: &crate::cli::AppSettings,
) -> Vec<ResourceChange> {
    resource_changes
        .into_iter()
        .filter(|change| {
            matches_filter(
                &change.resource_type,
                &settings.include_type,
                &settings.exclude_type,
            ) && matches_filter(
                &change.action,
                &settings.include_action,
                &settings.exclude_action,
            )
        })
        .collect()
}

pub fn matches_filter(value: &str, include: &[String], exclude: &[String]) -> bool {
    (include.is_empty()
        || include
            .iter()
            .any(|pattern| matches_pattern(value, pattern)))
        && !exclude
            .iter()
            .any(|pattern| matches_pattern(value, pattern))
}

pub fn matches_pattern(value: &str, pattern: &str) -> bool {
    Pattern::new(pattern).map_or_else(|_| pattern == value, |glob| glob.matches(value))
}

pub fn sort_resource_changes(changes: &mut [ResourceChange], sort_by: Option<crate::cli::SortBy>) {
    let Some(sort_by) = sort_by else {
        return;
    };
    changes.sort_by(|left, right| match sort_by {
        crate::cli::SortBy::Type => left
            .resource_type
            .cmp(&right.resource_type)
            .then_with(|| left.resource_name.cmp(&right.resource_name))
            .then_with(|| left.action.cmp(&right.action)),
        crate::cli::SortBy::Name => left
            .resource_name
            .cmp(&right.resource_name)
            .then_with(|| left.resource_type.cmp(&right.resource_type))
            .then_with(|| left.action.cmp(&right.action)),
        crate::cli::SortBy::Action => left
            .action
            .cmp(&right.action)
            .then_with(|| left.resource_type.cmp(&right.resource_type))
            .then_with(|| left.resource_name.cmp(&right.resource_name)),
    });
}

pub fn count_actions(resource_changes: &[ResourceChange]) -> ChangeCounts {
    let mut counts = ChangeCounts::default();
    for change in resource_changes {
        match change.action.as_str() {
            "create" => counts.create += 1,
            "update" => counts.update += 1,
            "delete" => counts.delete += 1,
            "replace" | "create/delete" => {
                counts.create += 1;
                counts.delete += 1;
            }
            _ => {}
        }
    }
    counts
}

pub fn has_fail_on_actions(resource_changes: &[ResourceChange], fail_on: &[String]) -> bool {
    fail_on.iter().any(|pattern| {
        resource_changes
            .iter()
            .any(|change| matches_pattern(&change.action, pattern))
    })
}
