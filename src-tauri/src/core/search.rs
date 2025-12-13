use std::cell::RefCell;

use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use rayon::prelude::*;

use crate::api::types::{SearchConfig, SearchMode, SearchScheme};

thread_local! {
    static MATCHER_PLAIN: RefCell<MatcherCtx> = RefCell::new(MatcherCtx::new(Config::DEFAULT));
    static MATCHER_PATH: RefCell<MatcherCtx> = RefCell::new(MatcherCtx::new(Config::DEFAULT.match_paths()));
}

struct MatcherCtx {
    matcher: Matcher,
    buf: Vec<char>,
}

impl MatcherCtx {
    fn new(cfg: Config) -> Self {
        Self {
            matcher: Matcher::new(cfg),
            buf: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rank {
    points: u64,
    index: usize,
}

impl Rank {
    fn new(score: u32, pathname: u32, length: usize, index: usize) -> Self {
        let score_inv = (u16::MAX as u32).saturating_sub(score.min(u16::MAX as u32)) as u16;
        let pathname = pathname.min(u16::MAX as u32) as u16;
        let length = length.min(u16::MAX as usize) as u16;
        let points = ((score_inv as u64) << 48)
            | ((pathname as u64) << 32)
            | ((length as u64) << 16);
        Self { points, index }
    }
}

fn cmp_rank(a: &Rank, b: &Rank) -> std::cmp::Ordering {
    match a.points.cmp(&b.points) {
        std::cmp::Ordering::Equal => a.index.cmp(&b.index),
        other => other,
    }
}

fn path_metrics(haystack: &str) -> (usize, Option<usize>) {
    let bytes = haystack.as_bytes();
    let mut last_delim = None;
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'/' || *b == b'\\' {
            last_delim = Some(i + 1);
        }
    }
    // Ignore trailing delimiter (directories)
    if matches!(bytes.last(), Some(b'/' | b'\\')) {
        last_delim = bytes
            .iter()
            .enumerate()
            .rev()
            .skip(1)
            .find_map(|(i, b)| (*b == b'/' || *b == b'\\').then_some(i + 1));
    }
    (bytes.len(), last_delim)
}

fn pathname_distance(last_delim: Option<usize>, min_begin: usize) -> u32 {
    match last_delim {
        Some(last) if last <= min_begin => (min_begin - last) as u32,
        _ => u16::MAX as u32,
    }
}

fn ascii_lower(b: u8) -> u8 {
    if (b'A'..=b'Z').contains(&b) {
        b + 32
    } else {
        b
    }
}

fn basename_substring_start(
    haystack: &str,
    query_lower: &[u8],
    query_is_ascii: bool,
    last_delim: Option<usize>,
) -> Option<usize> {
    let start = last_delim.unwrap_or(0);
    let hay_bytes = haystack.as_bytes();
    let hay = &hay_bytes[start..];

    if !query_is_ascii || !haystack.is_ascii() {
        // Fallback: let std handle casefolding; this path is colder.
        let hay_lower = haystack[start..].to_ascii_lowercase();
        return hay_lower
            .find(std::str::from_utf8(query_lower).unwrap_or(""))
            .map(|pos| start + pos);
    }

    // ASCII fast path: sliding window compare with lowercasing on the fly.
    hay.windows(query_lower.len()).position(|w| {
        w.iter()
            .zip(query_lower.iter())
            .all(|(hb, qb)| ascii_lower(*hb) == *qb)
    }).map(|pos| start + pos)
}

pub fn filter_items<'a>(
    items: &'a [String],
    query: &str,
    config: &SearchConfig,
) -> Vec<&'a String> {
    if query.is_empty() {
        return items.iter().collect();
    }

    let atom_kind = match config.mode {
        SearchMode::Exact => AtomKind::Substring,
        SearchMode::Fuzzy => AtomKind::Fuzzy,
    };

    let query_lower = query.to_ascii_lowercase();
    let query_bytes = query_lower.as_bytes();
    let query_is_ascii = query.is_ascii();

    let pattern = Pattern::new(query, CaseMatching::Ignore, Normalization::Smart, atom_kind);
    let field_idx = config.field.saturating_sub(1);
    let use_path = matches!(config.scheme, SearchScheme::Path);

    let mut ranked: Vec<_> = items
        .par_iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            let haystack = item.split('\t').nth(field_idx).unwrap_or(item);
            let matcher_tls = if use_path {
                &MATCHER_PATH
            } else {
                &MATCHER_PLAIN
            };
            matcher_tls.with(|cell| {
                let mut ctx = cell.borrow_mut();
                let MatcherCtx {
                    matcher,
                    buf,
                } = &mut *ctx;

                let haystack_str = Utf32Str::new(haystack, buf);

                let score = if use_path {
                    // We don't need full indices for tiebreaking; score() is cheaper.
                    pattern.score(haystack_str, matcher)
                } else {
                    pattern.score(haystack_str, matcher)
                }?;

                let (length, pathname) = if use_path {
                    let (len, last_delim) = path_metrics(haystack);
                    let begin =
                        basename_substring_start(haystack, query_bytes, query_is_ascii, last_delim)
                            .unwrap_or(0);
                    (len, pathname_distance(last_delim, begin))
                } else {
                    (haystack.len(), 0)
                };

                let rank = Rank::new(score, pathname, length, idx);
                Some((rank, item))
            })
        })
        .collect();

    ranked.sort_by(|a, b| cmp_rank(&a.0, &b.0));
    ranked.into_iter().map(|(_, item)| item).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::{SearchConfig, SearchMode, SearchScheme};

    #[test]
    fn pathname_distance_prefers_basename() {
        let path = "/a/b/file.rs";
        let (_, last) = path_metrics(path);
        assert_eq!(pathname_distance(last, 5), 0); // starts at filename
        assert_eq!(pathname_distance(last, 8), 3); // further into filename
        assert_eq!(pathname_distance(last, 1), u16::MAX as u32); // before last delimiter penalized
    }

    #[test]
    fn rank_sorting_matches_fzf_order() {
        let mut ranks = vec![
            (Rank::new(200, 2, 20, 1), "b"),
            (Rank::new(200, 1, 25, 0), "a"),
            (Rank::new(150, 5, 30, 2), "c"),
        ];

        ranks.sort_by(|a, b| cmp_rank(&a.0, &b.0));
        let ordered: Vec<_> = ranks.into_iter().map(|(_, label)| label).collect();
        assert_eq!(ordered, vec!["a", "b", "c"]);
    }

    #[test]
    fn path_scheme_prefers_basename_exact_match() {
        let items = vec![
            "FILE\tRepositoryPathFieldProperty.nib\t/Users/adrian/Downloads/Xcode.app/Contents/PlugIns/IDESourceControl.ideplugin/Contents/Resources/RepositoryPathFieldProperty.nib".to_string(),
            "FILE\tRepositoryBrowserViewController.nib\t/Users/adrian/Downloads/Xcode.app/Contents/PlugIns/IDESourceControl.ideplugin/Contents/Resources/RepositoryBrowserViewController.nib".to_string(),
            "FILE\trepository-request.graphql\t/Users/adrian/Downloads/Xcode.app/Contents/SharedFrameworks/XCSourceControl.framework/Versions/A/XPCServices/repository-request.graphql".to_string(),
            "DIR\trepos\t/Users/adrian/MEGA/repos/".to_string(),
        ];

        let config = SearchConfig {
            field: 3,
            scheme: SearchScheme::Path,
            mode: SearchMode::Fuzzy,
        };

        let results = filter_items(&items, "repos", &config);
        assert_eq!(
            results.first().map(|s| s.as_str()),
            Some("DIR\trepos\t/Users/adrian/MEGA/repos/")
        );
    }
}
