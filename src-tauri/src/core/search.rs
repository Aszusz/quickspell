use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use crate::api::types::{SearchConfig, SearchMode, SearchScheme};

pub fn filter_items<'a>(
    items: &'a [String],
    query: &str,
    config: &SearchConfig,
) -> Vec<&'a String> {
    if query.is_empty() {
        return items.iter().collect();
    }

    let matcher_config = match config.scheme {
        SearchScheme::Path => Config::DEFAULT.match_paths(),
        SearchScheme::Plain => Config::DEFAULT,
    };
    let mut matcher = Matcher::new(matcher_config);

    let atom_kind = match config.mode {
        SearchMode::Exact => AtomKind::Substring,
        SearchMode::Fuzzy => AtomKind::Fuzzy,
    };

    let pattern = Pattern::new(query, CaseMatching::Ignore, Normalization::Smart, atom_kind);
    let field_idx = config.field.saturating_sub(1); // convert 1-indexed to 0-indexed

    let mut scored: Vec<_> = items
        .iter()
        .filter_map(|item| {
            let haystack = item.split('\t').nth(field_idx).unwrap_or(item);
            let mut buf = Vec::new();
            let haystack_str = Utf32Str::new(haystack, &mut buf);
            pattern
                .score(haystack_str, &mut matcher)
                .map(|score| (score, item))
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, item)| item).collect()
}
