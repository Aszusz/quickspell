use crate::api::types::{SearchConfig, SearchMode, SearchScheme};
use crate::core::fuzzy;

pub fn filter_items<'a>(
    items: &'a [String],
    query: &str,
    config: &SearchConfig,
) -> Vec<&'a String> {
    let options = fuzzy::Options {
        field: config.field,
        scheme: match config.scheme {
            SearchScheme::Plain => fuzzy::Scheme::Default,
            SearchScheme::Path => fuzzy::Scheme::Path,
        },
        mode: match config.mode {
            SearchMode::Fuzzy => fuzzy::Mode::Fuzzy,
            SearchMode::Exact => fuzzy::Mode::Exact,
        },
    };

    fuzzy::filter_items(items, query, &options)
}
