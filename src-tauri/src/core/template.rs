use std::collections::HashMap;

use handlebars::Handlebars;
use serde::Serialize;

use crate::api::types::Frame;

#[derive(Debug, PartialEq, Eq)]
pub enum TemplateError {
    Render(String),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct SelectionContext {
    #[serde(rename = "type")]
    kind: String,
    label: String,
    data: String,
    fields: Vec<String>,
    raw: String,
}

impl SelectionContext {
    fn from_item(item: Option<&String>) -> Self {
        let (raw, fields) = match item {
            Some(value) => (value.clone(), split_fields(value)),
            None => (String::new(), Vec::new()),
        };

        Self {
            kind: fields.first().cloned().unwrap_or_default(),
            label: fields.get(1).cloned().unwrap_or_default(),
            data: fields.get(2).cloned().unwrap_or_default(),
            fields,
            raw,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct FrameContext {
    selection: SelectionContext,
    query: String,
    #[serde(rename = "spellId")]
    spell_id: String,
}

#[derive(Debug, Serialize)]
struct TemplateContext {
    context: HashMap<String, FrameContext>,
}

pub fn resolve_template(template: &str, frames: &[Frame]) -> Result<String, TemplateError> {
    let mut hb = Handlebars::new();
    hb.register_escape_fn(handlebars::no_escape);

    let data = TemplateContext {
        context: build_context(frames),
    };

    hb.render_template(template, &data)
        .map_err(|err| TemplateError::Render(err.to_string()))
}

fn build_context(frames: &[Frame]) -> HashMap<String, FrameContext> {
    let mut ctx = HashMap::new();

    for frame in frames {
        let selected = selected_item(frame);
        let selection = SelectionContext::from_item(selected);
        ctx.insert(
            frame.spell_id.clone(),
            FrameContext {
                selection,
                query: frame.query.clone(),
                spell_id: frame.spell_id.clone(),
            },
        );
    }

    ctx
}

fn selected_item(frame: &Frame) -> Option<&String> {
    if frame.filtered_items.is_empty() {
        return None;
    }
    let idx = frame
        .selected_idx
        .min(frame.filtered_items.len().saturating_sub(1));
    frame.filtered_items.get(idx)
}

fn split_fields(value: &str) -> Vec<String> {
    value.split('\t').map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(spell_id: &str, items: Vec<&str>, selected_idx: usize, query: &str) -> Frame {
        Frame {
            spell_id: spell_id.to_string(),
            query: query.to_string(),
            all_items: items.iter().map(|s| s.to_string()).collect(),
            filtered_items: items.iter().map(|s| s.to_string()).collect(),
            selected_idx,
        }
    }

    #[test]
    fn resolves_basic_field() {
        let frames = vec![frame(
            "search_files",
            vec!["FILE\t[F] notes.txt\t/Users/me/notes.txt"],
            0,
            "notes",
        )];

        let out = resolve_template("{{context.search_files.selection.data}}", &frames).unwrap();
        assert_eq!(out, "/Users/me/notes.txt");
    }

    #[test]
    fn resolves_condition_string() {
        let frames = vec![frame(
            "quickspell",
            vec!["APP\t[A] Notes\t/Applications/Notes.app"],
            0,
            "",
        )];

        let out =
            resolve_template("{{context.quickspell.selection.type}} == 'APP'", &frames).unwrap();
        assert_eq!(out, "APP == 'APP'");
    }

    #[test]
    fn handles_missing_selection() {
        let frames = vec![frame("search_files", Vec::new(), 0, "")];

        let out = resolve_template("{{context.search_files.selection.data}}", &frames).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn resolves_multi_frame_context() {
        let frames = vec![
            frame("quickspell", vec!["SPELL\tQuickspell\tsearch_files"], 0, ""),
            frame(
                "search_files",
                vec!["FILE\t[F] notes.txt\t/Users/me/notes.txt"],
                0,
                "notes",
            ),
        ];

        let out = resolve_template(
            "{{context.quickspell.selection.data}} -> {{context.search_files.selection.data}}",
            &frames,
        )
        .unwrap();

        assert_eq!(out, "search_files -> /Users/me/notes.txt");
    }
}
