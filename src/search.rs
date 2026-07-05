use crate::model::ProjectContract;
use serde::Serialize;
use strsim::jaro_winkler;

#[derive(Debug, Clone, Serialize)]
pub struct SearchMatch {
    pub name: String,
    pub description: String,
    pub score: f64,
}

pub fn search(projects: &[ProjectContract], query: &str, limit: usize) -> Vec<SearchMatch> {
    let q = normalize(query);
    let q_tokens: Vec<&str> = q.split_whitespace().collect();
    let mut matches: Vec<_> = projects
        .iter()
        .map(|contract| {
            let name = normalize(&contract.project.name.replace('-', " "));
            let description = normalize(&contract.project.description);
            let haystack = format!("{name} {description}");

            let token_hits = if q_tokens.is_empty() {
                0.0
            } else {
                q_tokens.iter().filter(|token| haystack.contains(**token)).count() as f64
                    / q_tokens.len() as f64
            };
            let exact_bonus = if contract.project.name == query { 1.0 } else { 0.0 };
            let contains_bonus = if haystack.contains(&q) { 0.15 } else { 0.0 };
            let similarity = jaro_winkler(&q, &name);
            let score = (0.55 * token_hits + 0.30 * similarity + contains_bonus + exact_bonus).min(1.0);

            SearchMatch {
                name: contract.project.name.clone(),
                description: contract.project.description.clone(),
                score,
            }
        })
        .filter(|m| q.is_empty() || m.score >= 0.20)
        .collect();

    matches.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.name.cmp(&b.name)));
    matches.truncate(limit);
    matches
}

fn normalize(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
