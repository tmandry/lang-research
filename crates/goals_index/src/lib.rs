//! Builds an index of Rust project goals from a checkout of
//! [rust-lang/goals](https://github.com/rust-lang/goals), and resolves the goal
//! references that posts make in their front matter.
//!
//! A goal is identified by its **tracking issue**. Goals that carry over between
//! periods reuse the same tracking issue, sometimes under a new slug (for example
//! `in-place-initialization` in 2025h2 became `in-place-init` in 2026). Authors
//! may refer to a goal by any slug it has ever had, without the period, and all
//! of those references end up on one goal page.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::Path;

use serde::Serialize;

/// A goal period such as `2025h2` or `2026`, ordered chronologically.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(into = "String")]
pub struct Period {
    year: u32,
    /// 0 for a whole-year period, otherwise 1 or 2.
    half: u8,
}

impl Period {
    pub fn parse(s: &str) -> Option<Period> {
        let (year, half) = match s.split_once('h') {
            Some((y, h)) => (y, h.parse().ok().filter(|h| *h == 1 || *h == 2)?),
            None => (s, 0),
        };
        if year.len() != 4 {
            return None;
        }
        Some(Period { year: year.parse().ok()?, half })
    }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.half {
            0 => write!(f, "{}", self.year),
            h => write!(f, "{}h{}", self.year, h),
        }
    }
}

impl From<Period> for String {
    fn from(p: Period) -> String {
        p.to_string()
    }
}

/// One goal document, i.e. a goal in one period.
#[derive(Clone, Debug)]
pub struct GoalDoc {
    pub period: Period,
    pub slug: String,
    pub title: String,
    pub issue: u64,
}

/// A goal across all of its periods.
#[derive(Clone, Debug, Serialize)]
pub struct Goal {
    pub issue: u64,
    /// URL path component for this goal's page on the blog.
    pub canonical_slug: String,
    /// Title of the most recent goal document.
    pub title: String,
    pub latest_period: Period,
    /// Slug used in each period.
    pub periods: BTreeMap<String, String>,
    pub doc_url: String,
    pub issue_url: String,
}

#[derive(Debug, Default, Serialize)]
pub struct GoalsIndex {
    /// Keyed by tracking issue number (as a string, for JSON).
    pub goals: BTreeMap<String, Goal>,
    /// Every accepted way of referring to a goal, lowercased, mapped to the
    /// tracking issue number. Ambiguous bare slugs are left out; see
    /// `ambiguous`.
    pub aliases: BTreeMap<String, String>,
    /// Bare slugs that refer to more than one goal, with the
    /// `slug@period` forms that disambiguate them.
    pub ambiguous: BTreeMap<String, Vec<String>>,
}

/// Errors from resolving a goal reference in a post.
#[derive(Debug, PartialEq, Eq)]
pub enum ResolveError {
    Unknown { reference: String, suggestions: Vec<String> },
    Ambiguous { reference: String, choices: Vec<String> },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::Unknown { reference, suggestions } => {
                write!(f, "unknown project goal `{reference}`")?;
                if !suggestions.is_empty() {
                    write!(f, "; did you mean one of: {}?", suggestions.join(", "))?;
                }
                write!(
                    f,
                    " (goals without a tracking issue can't be linked yet)"
                )
            }
            ResolveError::Ambiguous { reference, choices } => write!(
                f,
                "`{reference}` refers to more than one project goal; use one of: {}",
                choices.join(", ")
            ),
        }
    }
}

/// Parse a single goal document. Returns `None` if it isn't a goal with a
/// tracking issue (e.g. a README, or a goal that was never accepted).
pub fn parse_goal_doc(period: Period, slug: &str, text: &str) -> Option<GoalDoc> {
    let title = text
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .map(|t| t.trim().to_string())?;
    let issue = text.lines().find_map(|line| {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() < 3 || !cells[1].eq_ignore_ascii_case("tracking issue") {
            return None;
        }
        // e.g. `[rust-lang/goals#390]` or `[rust-lang/rust-project-goals#123]`
        let (_, after) = cells[2].split_once("goals#")?;
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse().ok()
    })?;
    Some(GoalDoc { period, slug: slug.to_string(), title, issue })
}

/// Read every goal document under `<goals_repo>/src/<period>/*.md`.
pub fn read_goal_docs(goals_repo: &Path) -> std::io::Result<Vec<GoalDoc>> {
    let mut docs = Vec::new();
    for entry in fs::read_dir(goals_repo.join("src"))? {
        let entry = entry?;
        let Some(period) = entry.file_name().to_str().and_then(Period::parse) else {
            continue;
        };
        if !entry.file_type()?.is_dir() {
            continue;
        }
        for file in fs::read_dir(entry.path())? {
            let path = file?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Some(slug) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let text = fs::read_to_string(&path)?;
            if let Some(doc) = parse_goal_doc(period.clone(), slug, &text) {
                docs.push(doc);
            }
        }
    }
    Ok(docs)
}

impl GoalsIndex {
    pub fn build(docs: &[GoalDoc]) -> GoalsIndex {
        // Group documents by tracking issue.
        let mut by_issue: BTreeMap<u64, Vec<&GoalDoc>> = BTreeMap::new();
        for doc in docs {
            by_issue.entry(doc.issue).or_default().push(doc);
        }

        // The most recent document for each issue decides its slug and title.
        let mut latest: BTreeMap<u64, &GoalDoc> = BTreeMap::new();
        for (issue, docs) in &by_issue {
            let doc = docs.iter().max_by_key(|d| &d.period).unwrap();
            latest.insert(*issue, doc);
        }

        // Assign canonical slugs. If two goals end up with the same latest slug,
        // the more recent goal keeps it and older ones get `-<period>` appended.
        let mut by_slug: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        for (issue, doc) in &latest {
            by_slug.entry(doc.slug.to_lowercase()).or_default().push(*issue);
        }
        let mut canonical: BTreeMap<u64, String> = BTreeMap::new();
        for (slug, mut issues) in by_slug {
            issues.sort_by_key(|i| std::cmp::Reverse(latest[i].period.clone()));
            for (n, issue) in issues.into_iter().enumerate() {
                let name = if n == 0 {
                    slug.clone()
                } else {
                    format!("{slug}-{}", latest[&issue].period)
                };
                canonical.insert(issue, name);
            }
        }

        let mut index = GoalsIndex::default();
        // slug -> set of issues, to detect ambiguity across all periods.
        let mut slug_issues: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();

        for (issue, docs) in &by_issue {
            let doc = latest[issue];
            let periods = docs
                .iter()
                .map(|d| (d.period.to_string(), d.slug.clone()))
                .collect();
            index.goals.insert(
                issue.to_string(),
                Goal {
                    issue: *issue,
                    canonical_slug: canonical[issue].clone(),
                    title: doc.title.clone(),
                    latest_period: doc.period.clone(),
                    periods,
                    doc_url: format!("https://goals.rust-lang.org/{}/{}.html", doc.period, doc.slug),
                    issue_url: format!("https://github.com/rust-lang/goals/issues/{issue}"),
                },
            );

            let id = issue.to_string();
            index.aliases.insert(id.clone(), id.clone());
            index.aliases.insert(format!("goals#{issue}"), id.clone());
            index.aliases.insert(canonical[issue].clone(), id.clone());
            for d in docs {
                let slug = d.slug.to_lowercase();
                index.aliases.insert(format!("{slug}@{}", d.period), id.clone());
                slug_issues.entry(slug).or_default().insert(*issue);
            }
        }

        for (slug, issues) in slug_issues {
            if issues.len() == 1 {
                let issue = issues.into_iter().next().unwrap().to_string();
                index.aliases.entry(slug).or_insert(issue);
            } else {
                let mut choices: Vec<String> = docs
                    .iter()
                    .filter(|d| d.slug.to_lowercase() == slug)
                    .map(|d| format!("{slug}@{}", d.period))
                    .collect();
                choices.sort();
                choices.dedup();
                index.ambiguous.insert(slug, choices);
            }
        }
        // A bare slug that is ambiguous across periods must not resolve, even if
        // it happens to be one goal's canonical slug: the author may well have
        // meant the older goal.
        for slug in index.ambiguous.keys() {
            index.aliases.remove(slug);
        }
        index
    }

    pub fn resolve(&self, reference: &str) -> Result<&Goal, ResolveError> {
        let key = reference.trim().trim_start_matches('#').to_lowercase();
        if let Some(choices) = self.ambiguous.get(&key) {
            return Err(ResolveError::Ambiguous {
                reference: reference.to_string(),
                choices: choices.clone(),
            });
        }
        match self.aliases.get(&key) {
            Some(issue) => Ok(&self.goals[issue]),
            None => Err(ResolveError::Unknown {
                reference: reference.to_string(),
                suggestions: self.suggest(&key),
            }),
        }
    }

    fn suggest(&self, key: &str) -> Vec<String> {
        let mut scored: Vec<(usize, &str)> = self
            .aliases
            .keys()
            .filter(|a| !a.contains('@') && !a.starts_with(|c: char| c.is_ascii_digit()) && !a.starts_with("goals#"))
            .map(|a| (strsim::levenshtein(key, a), a.as_str()))
            .filter(|(d, _)| *d <= key.len().max(3) / 2)
            .collect();
        scored.sort();
        scored.into_iter().take(3).map(|(_, a)| a.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(period: &str, slug: &str, issue: u64) -> GoalDoc {
        GoalDoc {
            period: Period::parse(period).unwrap(),
            slug: slug.into(),
            title: format!("Title of {slug} ({period})"),
            issue,
        }
    }

    fn sample() -> GoalsIndex {
        GoalsIndex::build(&[
            // Renamed between periods.
            doc("2025h2", "in-place-initialization", 395),
            doc("2026", "in-place-init", 395),
            // Case-only rename.
            doc("2024h2", "Polonius", 118),
            doc("2025h1", "polonius", 118),
            // Same slug, different goals.
            doc("2024h2", "macro-improvements", 252),
            doc("2026", "macro-improvements", 629),
            // Plain carried-over goal.
            doc("2025h2", "field-projections", 390),
            doc("2026", "field-projections", 390),
        ])
    }

    #[test]
    fn periods_order_chronologically() {
        let p = |s| Period::parse(s).unwrap();
        assert!(p("2024h2") < p("2025h1"));
        assert!(p("2025h2") < p("2026"));
        assert_eq!(p("2025h2").to_string(), "2025h2");
        assert!(Period::parse("faq").is_none());
        assert!(Period::parse("2025h3").is_none());
    }

    #[test]
    fn renamed_goal_resolves_from_any_slug() {
        let idx = sample();
        for r in ["in-place-init", "in-place-initialization", "IN-PLACE-INIT", "goals#395", "395", "#395"] {
            let g = idx.resolve(r).unwrap();
            assert_eq!(g.issue, 395, "{r}");
            assert_eq!(g.canonical_slug, "in-place-init");
        }
    }

    #[test]
    fn case_only_rename_is_one_goal() {
        let idx = sample();
        assert_eq!(idx.resolve("Polonius").unwrap().issue, 118);
        assert_eq!(idx.resolve("polonius").unwrap().canonical_slug, "polonius");
    }

    #[test]
    fn ambiguous_slug_needs_period() {
        let idx = sample();
        let err = idx.resolve("macro-improvements").unwrap_err();
        assert_eq!(
            err,
            ResolveError::Ambiguous {
                reference: "macro-improvements".into(),
                choices: vec!["macro-improvements@2024h2".into(), "macro-improvements@2026".into()],
            }
        );
        assert_eq!(idx.resolve("macro-improvements@2026").unwrap().issue, 629);
        assert_eq!(idx.resolve("macro-improvements@2024h2").unwrap().issue, 252);
        // Canonical slugs stay distinct, so the two goals get separate pages.
        assert_eq!(idx.goals["629"].canonical_slug, "macro-improvements");
        assert_eq!(idx.goals["252"].canonical_slug, "macro-improvements-2024h2");
    }

    #[test]
    fn unknown_slug_suggests_close_matches() {
        let idx = sample();
        match idx.resolve("field-projection").unwrap_err() {
            ResolveError::Unknown { suggestions, .. } => {
                assert_eq!(suggestions.first().map(String::as_str), Some("field-projections"))
            }
            e => panic!("unexpected {e:?}"),
        }
    }

    #[test]
    fn parses_goal_doc_metadata_table() {
        let text = "# Field projections\n\n| Metadata | |\n|---|---|\n| Point of contact | @BennoLossin |\n| Tracking issue       | [rust-lang/goals#390]        |\n";
        let d = parse_goal_doc(Period::parse("2026").unwrap(), "field-projections", text).unwrap();
        assert_eq!(d.issue, 390);
        assert_eq!(d.title, "Field projections");

        let old = "# Old\n| Tracking issue | [rust-lang/rust-project-goals#123] |\n";
        assert_eq!(parse_goal_doc(Period::parse("2024h2").unwrap(), "old", old).unwrap().issue, 123);

        let none = "# Not accepted\n| Point of contact | @x |\n";
        assert!(parse_goal_doc(Period::parse("2026").unwrap(), "x", none).is_none());
    }
}
