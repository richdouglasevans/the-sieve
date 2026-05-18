use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::ast::{Alignment, Document, Element, Inline, ListItem, Table};
use crate::error::Result;
use crate::parser::extensions::{
    detect_code_block_type, is_page_break, parse_boxed_text, parse_column_layout,
    parse_image_attributes, parse_license, CodeBlockType,
};

/// Parse markdown content into our intermediate AST
pub fn parse(content: &str) -> Result<Document> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(content, options);
    let mut document = Document::new();
    let mut state = ParserState::new();

    for event in parser {
        process_event(event, &mut document, &mut state)?;
    }

    Ok(document)
}

struct ParserState {
    /// Stack of inline content being built
    inline_stack: Vec<Vec<Inline>>,
    /// Current code block info
    code_block_type: Option<CodeBlockType>,
    code_block_content: String,
    /// List state
    list_stack: Vec<ListState>,
    /// Table state
    table_state: Option<TableState>,
    /// Blockquote content
    blockquote_content: Vec<Element>,
    /// Track nesting level
    in_blockquote: bool,
}

struct ListState {
    ordered: bool,
    items: Vec<ListItem>,
    current_item: Vec<Element>,
    current_task: Option<bool>,
}

struct TableState {
    headers: Vec<String>,
    alignments: Vec<Alignment>,
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: String,
    in_head: bool,
}

impl ParserState {
    fn new() -> Self {
        Self {
            inline_stack: Vec::new(),
            code_block_type: None,
            code_block_content: String::new(),
            list_stack: Vec::new(),
            table_state: None,
            blockquote_content: Vec::new(),
            in_blockquote: false,
        }
    }

    fn push_inline_context(&mut self) {
        self.inline_stack.push(Vec::new());
    }

    fn pop_inline_context(&mut self) -> Vec<Inline> {
        self.inline_stack.pop().unwrap_or_default()
    }

    fn current_inlines(&mut self) -> &mut Vec<Inline> {
        if self.inline_stack.is_empty() {
            self.inline_stack.push(Vec::new());
        }
        self.inline_stack.last_mut().unwrap()
    }

    fn add_inline(&mut self, inline: Inline) {
        self.current_inlines().push(inline);
    }
}

fn process_event(event: Event, document: &mut Document, state: &mut ParserState) -> Result<()> {
    match event {
        // Block-level events
        Event::Start(tag) => process_start_tag(tag, state),
        Event::End(tag) => process_end_tag(tag, document, state),

        // Inline text
        Event::Text(text) => {
            if state.code_block_type.is_some() {
                state.code_block_content.push_str(&text);
            } else if let Some(ref mut table) = state.table_state {
                table.current_cell.push_str(&text);
            } else {
                state.add_inline(Inline::Text(text.to_string()));
            }
        }

        Event::Code(code) => {
            state.add_inline(Inline::Code(code.to_string()));
        }

        Event::SoftBreak => {
            if state.code_block_type.is_some() {
                state.code_block_content.push('\n');
            } else {
                state.add_inline(Inline::SoftBreak);
            }
        }

        Event::HardBreak => {
            state.add_inline(Inline::HardBreak);
        }

        Event::Html(html) => {
            if is_page_break(&html) {
                document.push(Element::PageBreak);
            } else if let Some(columns) = parse_column_layout(&html) {
                document.push(Element::ColumnLayout(columns));
            } else if let Some((kind, info)) = parse_license(&html) {
                document.push(Element::License { kind, info });
            } else {
                document.push(Element::Raw(html.to_string()));
            }
        }

        Event::InlineHtml(html) => {
            state.add_inline(Inline::Text(html.to_string()));
        }

        Event::Rule => {
            document.push(Element::ThematicBreak);
        }

        Event::TaskListMarker(checked) => {
            if let Some(list) = state.list_stack.last_mut() {
                list.current_task = Some(checked);
            }
        }

        Event::FootnoteReference(_) => {
            // Not yet supported
        }
    }
    Ok(())
}

fn process_start_tag(tag: Tag, state: &mut ParserState) {
    match tag {
        Tag::Paragraph => {
            state.push_inline_context();
        }
        Tag::Heading { .. } => {
            state.push_inline_context();
        }
        Tag::CodeBlock(kind) => {
            let info = match &kind {
                CodeBlockKind::Fenced(info) => info.as_ref(),
                CodeBlockKind::Indented => "",
            };
            state.code_block_type = Some(detect_code_block_type(info));
            state.code_block_content.clear();
        }
        Tag::BlockQuote => {
            state.in_blockquote = true;
            state.blockquote_content.clear();
        }
        Tag::List(start) => {
            // If we're inside a list item with pending inline content, save it first
            // This ensures "- Text:\n  - nested" keeps "Text:" before the nested list
            let pending_inlines = if !state.list_stack.is_empty() && !state.inline_stack.is_empty() {
                let inlines = state.pop_inline_context();
                // Push empty context back for the parent item to maintain stack balance
                state.push_inline_context();
                if !inlines.is_empty() { Some(inlines) } else { None }
            } else {
                None
            };

            if let (Some(inlines), Some(parent_list)) = (pending_inlines, state.list_stack.last_mut()) {
                parent_list.current_item.push(Element::Paragraph(inlines));
            }

            state.list_stack.push(ListState {
                ordered: start.is_some(),
                items: Vec::new(),
                current_item: Vec::new(),
                current_task: None,
            });
        }
        Tag::Item => {
            state.push_inline_context();
        }
        Tag::Table(alignments) => {
            state.table_state = Some(TableState {
                headers: Vec::new(),
                alignments: alignments
                    .iter()
                    .map(|a| match a {
                        pulldown_cmark::Alignment::Left => Alignment::Left,
                        pulldown_cmark::Alignment::Center => Alignment::Center,
                        pulldown_cmark::Alignment::Right => Alignment::Right,
                        pulldown_cmark::Alignment::None => Alignment::None,
                    })
                    .collect(),
                rows: Vec::new(),
                current_row: Vec::new(),
                current_cell: String::new(),
                in_head: false,
            });
        }
        Tag::TableHead => {
            if let Some(ref mut table) = state.table_state {
                table.in_head = true;
            }
        }
        Tag::TableRow => {
            if let Some(ref mut table) = state.table_state {
                table.current_row.clear();
            }
        }
        Tag::TableCell => {
            if let Some(ref mut table) = state.table_state {
                table.current_cell.clear();
            }
        }
        Tag::Emphasis => {
            state.push_inline_context();
        }
        Tag::Strong => {
            state.push_inline_context();
        }
        Tag::Link { dest_url, .. } => {
            state.push_inline_context();
            // Store URL for later - we'll handle this in End tag
            state.add_inline(Inline::Text(format!("\0LINK:{}\0", dest_url)));
        }
        Tag::Image { dest_url, title, .. } => {
            state.push_inline_context();
            // Store image info for later
            state.add_inline(Inline::Text(format!(
                "\0IMAGE:{}:{}\0",
                dest_url,
                title.as_ref()
            )));
        }
        _ => {}
    }
}

fn process_end_tag(tag: TagEnd, document: &mut Document, state: &mut ParserState) {
    match tag {
        TagEnd::Paragraph => {
            let inlines = state.pop_inline_context();
            if !inlines.is_empty() {
                if state.in_blockquote {
                    state
                        .blockquote_content
                        .push(Element::Paragraph(inlines));
                } else if let Some(list) = state.list_stack.last_mut() {
                    list.current_item.push(Element::Paragraph(inlines));
                } else {
                    document.push(Element::Paragraph(inlines));
                }
            }
        }
        TagEnd::Heading(level) => {
            let inlines = state.pop_inline_context();
            let text = inlines_to_string(&inlines);
            let level = level as u8;
            document.push(Element::Heading { level, text });
        }
        TagEnd::CodeBlock => {
            if let Some(block_type) = state.code_block_type.take() {
                let content = std::mem::take(&mut state.code_block_content);
                let element = match block_type {
                    CodeBlockType::StatBlock => Element::StatBlock(content.trim().to_string()),
                    CodeBlockType::Boxed => Element::BoxedText(parse_boxed_text(&content)),
                    CodeBlockType::Regular(lang) => Element::CodeBlock {
                        language: lang,
                        code: content,
                    },
                };
                document.push(element);
            }
        }
        TagEnd::BlockQuote => {
            state.in_blockquote = false;
            let content = std::mem::take(&mut state.blockquote_content);
            if !content.is_empty() {
                document.push(Element::BlockQuote(content));
            }
        }
        TagEnd::List(_) => {
            if let Some(list_state) = state.list_stack.pop() {
                let element = Element::List {
                    ordered: list_state.ordered,
                    items: list_state.items,
                };
                if let Some(parent) = state.list_stack.last_mut() {
                    parent.current_item.push(element);
                } else {
                    document.push(element);
                }
            }
        }
        TagEnd::Item => {
            let inlines = state.pop_inline_context();
            if let Some(list) = state.list_stack.last_mut() {
                // If we have inlines, wrap them in a paragraph
                if !inlines.is_empty() {
                    list.current_item.push(Element::Paragraph(inlines));
                }
                let content = std::mem::take(&mut list.current_item);
                let task = list.current_task.take();
                list.items.push(ListItem { content, task });
            }
        }
        TagEnd::Table => {
            if let Some(table_state) = state.table_state.take() {
                document.push(Element::Table(Table {
                    headers: table_state.headers,
                    alignments: table_state.alignments,
                    rows: table_state.rows,
                }));
            }
        }
        TagEnd::TableHead => {
            if let Some(ref mut table) = state.table_state {
                table.in_head = false;
                table.headers = std::mem::take(&mut table.current_row);
            }
        }
        TagEnd::TableRow => {
            if let Some(ref mut table) = state.table_state {
                if !table.in_head {
                    let row = std::mem::take(&mut table.current_row);
                    table.rows.push(row);
                }
            }
        }
        TagEnd::TableCell => {
            if let Some(ref mut table) = state.table_state {
                let cell = std::mem::take(&mut table.current_cell);
                table.current_row.push(cell);
            }
        }
        TagEnd::Emphasis => {
            let inlines = state.pop_inline_context();
            state.add_inline(Inline::Emphasis(inlines));
        }
        TagEnd::Strong => {
            let inlines = state.pop_inline_context();
            state.add_inline(Inline::Strong(inlines));
        }
        TagEnd::Link => {
            let inlines = state.pop_inline_context();
            // Extract URL from the marker we placed
            let (url, text) = extract_link_info(&inlines);
            state.add_inline(Inline::Link {
                text,
                url: url.to_string(),
            });
        }
        TagEnd::Image => {
            let inlines = state.pop_inline_context();
            // Extract image info from the marker
            if let Some((path, title, alt)) = extract_image_info(&inlines) {
                let image = parse_image_attributes(&alt, &path, Some(&title));
                state.add_inline(Inline::Image(image));
            }
        }
        _ => {}
    }
}

fn inlines_to_string(inlines: &[Inline]) -> String {
    let mut result = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) => {
                // Skip our markers
                if !text.starts_with('\0') {
                    result.push_str(text);
                }
            }
            Inline::Emphasis(inner) => result.push_str(&inlines_to_string(inner)),
            Inline::Strong(inner) => result.push_str(&inlines_to_string(inner)),
            Inline::Code(code) => result.push_str(code),
            Inline::Link { text, .. } => result.push_str(&inlines_to_string(text)),
            Inline::SoftBreak | Inline::HardBreak => result.push(' '),
            Inline::Image(_) => {}
        }
    }
    result
}

fn extract_link_info(inlines: &[Inline]) -> (String, Vec<Inline>) {
    let mut url = String::new();
    let mut text = Vec::new();

    for inline in inlines {
        if let Inline::Text(t) = inline {
            if t.starts_with("\0LINK:") && t.ends_with('\0') {
                url = t[6..t.len() - 1].to_string();
                continue;
            }
        }
        text.push(inline.clone());
    }

    (url, text)
}

fn extract_image_info(inlines: &[Inline]) -> Option<(String, String, String)> {
    for inline in inlines {
        if let Inline::Text(t) = inline {
            if t.starts_with("\0IMAGE:") && t.ends_with('\0') {
                let content = &t[7..t.len() - 1];
                if let Some((path, title)) = content.split_once(':') {
                    return Some((path.to_string(), title.to_string(), String::new()));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let content = "# Hello World\n\nThis is a paragraph.";
        let doc = parse(content).unwrap();
        assert_eq!(doc.elements.len(), 2);
    }

    #[test]
    fn test_page_break() {
        let content = "Before\n\n<!-- pagebreak -->\n\nAfter";
        let doc = parse(content).unwrap();
        assert!(doc.elements.iter().any(|e| matches!(e, Element::PageBreak)));
    }
}
