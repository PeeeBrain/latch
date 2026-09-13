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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::Entry;

    fn workspace_with(entry: Entry) -> Workspace {
        let mut workspace = Workspace::new();
        workspace.credentials.push(entry);
        workspace.start([1u8; 32]);
        workspace
    }

    fn entry(totp_secret: Option<String>) -> Entry {
        Entry {
            id: "entry-1".to_string(),
            title: "Example".to_string(),
            username: "user".to_string(),
            password: "secret".to_string(),
            url: None,
            icon_url: None,
            totp_secret,
        }
    }

    #[test]
    fn preview_exposes_totp_presence_without_the_secret() {
        let mut workspace = workspace_with(entry(Some("GEZDGNBVGY3TQOJQ".to_string())));

        let results = search(&mut workspace, "").unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].has_totp);
        let json = serde_json::to_string(&results[0]).unwrap();
        assert!(!json.contains("GEZDGNBVGY3TQOJQ"));
        assert!(!json.contains("totp_secret"));
    }

    #[test]
    fn preview_marks_missing_totp() {
        let mut workspace = workspace_with(entry(None));

        let results = search(&mut workspace, "").unwrap();

        assert!(!results[0].has_totp);
    }
}
