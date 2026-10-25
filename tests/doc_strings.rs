//! The library's doc strings against the published site: every citation
//! names a page under `doc/public/`, and its fragment one of the page's
//! headings. The published crate carries no `doc/`, so this half of the
//! library check runs here rather than in `library::check`.

use std::path::{Path, PathBuf};

/// The anchor the site's `toc` extension gives a heading: punctuation
/// removed, lowercased, each run of spaces and hyphens one `-`; an explicit
/// `{#id}` wins.
fn slug(heading: &str) -> String {
    if let Some(open) = heading.rfind("{#")
        && let Some(id) = heading[open + 2..].strip_suffix('}')
    {
        return id.to_owned();
    }
    let kept: String = heading
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || c.is_whitespace())
        .collect::<String>()
        .trim()
        .to_lowercase();
    let mut slug = String::new();
    for c in kept.chars() {
        if c == '-' || c.is_whitespace() {
            if !slug.ends_with('-') {
                slug.push('-');
            }
        } else {
            slug.push(c);
        }
    }
    slug
}

/// The page a citation's path names: `path.md`, or a folder's `index.md`.
fn page(public: &Path, path: &str) -> Option<PathBuf> {
    let file = public.join(format!("{path}.md"));
    let folder = public.join(path).join("index.md");
    [file, folder]
        .into_iter()
        .find(|candidate| candidate.is_file())
}

#[test]
fn every_citation_names_a_published_page_and_heading() {
    let public = Path::new(env!("CARGO_MANIFEST_DIR")).join("doc/public");
    for (module, target) in hql::library::citations() {
        let path = target.strip_prefix('/').unwrap_or_else(|| {
            panic!("{module} cites `[[{target}]]`, which is not a path from the site's root")
        });
        let (path, fragment) = path
            .split_once('#')
            .map_or((path, None), |(p, f)| (p, Some(f)));
        let path = path.trim_end_matches('/');
        let page = page(&public, path).unwrap_or_else(|| {
            panic!("{module} cites `[[{target}]]`, which names no published page")
        });
        if let Some(fragment) = fragment {
            let text = std::fs::read_to_string(&page).unwrap();
            let anchors: Vec<String> = text
                .lines()
                .filter_map(|line| line.trim_start().strip_prefix('#'))
                .map(|heading| slug(heading.trim_start_matches('#')))
                .collect();
            assert!(
                anchors.iter().any(|anchor| anchor == fragment),
                "{module} cites `[[{target}]]`, and {} has no heading `#{fragment}`",
                page.display()
            );
        }
    }
}

#[test]
fn a_heading_anchor_is_its_slug() {
    assert_eq!(slug(" Division and rounding"), "division-and-rounding");
    assert_eq!(slug(" `Int` and `Float`: values"), "int-and-float-values");
    assert_eq!(slug(" range {#range}"), "range");
}
