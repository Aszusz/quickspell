use std::cell::RefCell;

use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use rayon::prelude::*;

use crate::api::types::{SearchConfig, SearchMode, SearchScheme};

thread_local! {
    static MATCHER_PLAIN: RefCell<Matcher> = RefCell::new(Matcher::new(Config::DEFAULT));
    static MATCHER_PATH: RefCell<Matcher> = RefCell::new(Matcher::new(Config::DEFAULT.match_paths()));
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

    let pattern = Pattern::new(query, CaseMatching::Ignore, Normalization::Smart, atom_kind);
    let field_idx = config.field.saturating_sub(1);
    let use_path = matches!(config.scheme, SearchScheme::Path);

    let mut scored: Vec<_> = items
        .par_iter()
        .filter_map(|item| {
            let haystack = item.split('\t').nth(field_idx).unwrap_or(item);
            let matcher_tls = if use_path {
                &MATCHER_PATH
            } else {
                &MATCHER_PLAIN
            };
            matcher_tls.with(|m| {
                let mut matcher = m.borrow_mut();
                let mut buf = Vec::new();
                let haystack_str = Utf32Str::new(haystack, &mut buf);
                pattern
                    .score(haystack_str, &mut matcher)
                    .map(|score| (score, item))
            })
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, item)| item).collect()
}
