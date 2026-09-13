use super::{workspace::Workspace, EntryPreview};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::cmp::Reverse;

pub fn search(workspace: &mut Workspace, query: &str) -> Result<Vec<EntryPreview>, String> {
    workspace.check_session()?;
    workspace.refresh();

    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, EntryPreview)> = workspace
        .credentials
        .iter()
        .filter_map(|entry| {
            if query.is_empty() {
                return Some((0, entry.into()));
            }
            let t = matcher.fuzzy_match(&entry.title, query).unwrap_or(0);
            let u = matcher.fuzzy_match(&entry.username, query).unwrap_or(0);
            let best = t.max(u);
            if best >= 50 {
                Some((best, entry.into()))
            } else {
                None
            }
        })
        .collect();

    scored.sort_by_key(|entry| Reverse(entry.0));
    Ok(scored.into_iter().map(|(_, p)| p).collect())
}
