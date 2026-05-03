use std::path::Path;

use crate::ast::{Alignment, Document, Element, Image, Inline, ListItem, Table};
use crate::licenses::{self, LicenseFragment};

/// Render a document AST to HTML for debugging the AST → HTML pipeline.
/// Not paginated; intended for browser viewing.
pub fn render_to_html(document: &Document, _base_path: &Path) -> String {
    let mut output = String::new();
    let mut in_single_column = false;

    output.push_str(&generate_html_preamble());

    output.push_str("<body>\n");
    output.push_str("<div class=\"content two-column\">\n");

    for element in &document.elements {
        match element {
            Element::ColumnLayout(cols) => {
                if *cols == 1 && !in_single_column {
                    output.push_str("</div>\n<div class=\"content single-column\">\n");
                    in_single_column = true;
                } else if *cols == 2 && in_single_column {
                    output.push_str("</div>\n<div class=\"content two-column\">\n");
                    in_single_column = false;
                }
            }
            Element::PageBreak => {
                output.push_str("<hr class=\"page-break\">\n");
            }
            Element::License { kind, info } => {
                output.push_str("<section class=\"license-section\">\n");
                output.push_str(&format!(
                    "<div class=\"license-title\">{}</div>\n",
                    escape_html(licenses::title(*kind))
                ));
                if let Some(attribution) = &info.attribution {
                    output.push_str(&format!(
                        "<p class=\"license-attribution\">{} Licensed under {}. To view a copy of this license, visit {}.</p>\n",
                        escape_html(attribution),
                        escape_html(licenses::short_name(*kind)),
                        escape_html(licenses::url(*kind)),
                    ));
                }
                if let Some(changes) = &info.changes {
                    output.push_str(&format!(
                        "<p class=\"license-changes\">Changes from original: {}</p>\n",
                        escape_html(changes)
                    ));
                }
                output.push_str("<div class=\"license\">\n");
                for frag in licenses::fragments(*kind) {
                    match frag {
                        LicenseFragment::Heading2(t) => {
                            output.push_str(&format!("<h2>{}</h2>\n", escape_html(&t)))
                        }
                        LicenseFragment::Heading3(t) => {
                            output.push_str(&format!("<h3>{}</h3>\n", escape_html(&t)))
                        }
                        LicenseFragment::Paragraph(t) => {
                            output.push_str(&format!("<p>{}</p>\n", escape_html(&t)))
                        }
                    }
                }
                output.push_str("</div>\n");
                output.push_str("</section>\n");
            }
            _ => {
                output.push_str(&render_element(element));
            }
        }
    }

    output.push_str("</div>\n");
    output.push_str("</body>\n</html>\n");

    output
}

fn generate_html_preamble() -> String {
    r##"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<style>
body {
  font-family: Palatino, "Palatino Linotype", Georgia, serif;
  font-size: 9pt;
  line-height: 1.4;
  margin: 1em;
  padding: 0;
}

.content.two-column {
  column-count: 2;
  column-gap: 11pt;
}

.content.single-column {
  column-count: 1;
}

h1 {
  column-span: all;
  text-align: center;
  font-size: 18pt;
  font-weight: bold;
  margin: 0.5em 0 0.8em 0;
}

h2 {
  font-size: 12pt;
  font-weight: bold;
  margin: 1em 0 0.6em 0;
}

h3 {
  font-size: 10pt;
  font-weight: bold;
  margin: 0.8em 0 0.5em 0;
}

h4 {
  font-size: 9pt;
  font-weight: bold;
  margin: 0.6em 0 0.4em 0;
}

p {
  margin: 0 0 0.6em 0;
  text-align: left;
}

ul, ol {
  margin: 0 0 0.6em 0;
  padding-left: 1.5em;
}

li {
  margin-bottom: 0.3em;
}

ul ul, ol ul {
  list-style-type: circle;
}

li:has(> ul:first-child):has(> ul:last-child),
li:has(> ol:first-child):has(> ol:last-child) {
  list-style-type: none;
}

.stat-block {
  background-color: #e8e8e8;
  padding: 8pt;
  border-radius: 2pt;
  margin: 0.5em 0;
}

.stat-block ul {
  margin: 0.2em 0 0 0;
  padding-left: 1.2em;
}

.stat-block li {
  margin-bottom: 0.1em;
}

.boxed-text {
  background-color: #f4f4f0;
  border: 0.5pt solid #999;
  padding: 8pt;
  margin: 0.5em 0;
  font-style: italic;
}

pre {
  font-family: monospace;
  font-size: 8pt;
  background-color: #f5f5f5;
  padding: 8pt;
  overflow-x: auto;
}

code {
  font-family: monospace;
  font-size: 8pt;
  background-color: #f5f5f5;
  padding: 1pt 3pt;
}

table {
  border-collapse: collapse;
  width: 100%;
  margin: 0.5em 0;
  font-size: 9pt;
}

th, td {
  border: 0.5pt solid #999;
  padding: 4pt 6pt;
  text-align: left;
}

th {
  background-color: #e8e8e8;
  font-weight: bold;
}

blockquote {
  margin: 0.5em 0 0.5em 1em;
  padding-left: 0.5em;
  border-left: 2pt solid #999;
  font-style: italic;
}

a {
  color: #333;
  text-decoration: underline;
}

img {
  max-width: 100%;
  height: auto;
}

hr {
  border: none;
  border-top: 1pt solid #999;
  margin: 1em 0;
}

/* Visible separator for explicit page breaks (HTML is not paginated). */
hr.page-break {
  border-top: 2pt dashed #999;
  margin: 1.5em 0;
}

.license-section {
  margin-top: 2em;
}

.license-title {
  text-align: center;
  font-size: 13pt;
  font-weight: bold;
  margin: 0.5em 0 0.5em 0;
}

.license-attribution {
  text-align: center;
  font-size: 9pt;
  font-weight: bold;
  margin: 0.4em 0;
}

.license-changes {
  text-align: center;
  font-size: 9pt;
  font-style: italic;
  margin: 0.3em 0 0.6em 0;
}

.license {
  font-size: 6.5pt;
  line-height: 1.3;
  column-count: 2;
  column-gap: 11pt;
}

.license p {
  font-size: 6.5pt;
  margin: 0 0 0.4em 0;
}

.license h2 {
  font-size: 8pt;
  font-weight: bold;
  margin: 0.6em 0 0.2em 0;
}

.license h3 {
  font-size: 7pt;
  font-weight: bold;
  margin: 0.4em 0 0.2em 0;
}
</style>
</head>
"##.to_string()
}

fn render_element(element: &Element) -> String {
    match element {
        Element::Heading { level, text } => {
            format!("<h{}>{}</h{}>\n", level, escape_html(text), level)
        }

        Element::Paragraph(inlines) => {
            format!("<p>{}</p>\n", render_inlines(inlines))
        }

        Element::CodeBlock { language, code } => {
            let lang_class = language.as_ref().map(|l| format!(" class=\"language-{}\"", l)).unwrap_or_default();
            format!("<pre><code{}>{}</code></pre>\n", lang_class, escape_html(code))
        }

        Element::BlockQuote(elements) => {
            let content: String = elements.iter().map(render_element).collect();
            format!("<blockquote>{}</blockquote>\n", content)
        }

        Element::List { ordered, items } => {
            render_list(*ordered, items)
        }

        Element::ThematicBreak => "<hr>\n".to_string(),

        Element::PageBreak => "<div class=\"page-break\"></div>\n".to_string(),
        Element::License { .. } => String::new(), // handled in render_to_html main loop

        Element::ColumnLayout(_) => String::new(), // Handled in main loop

        Element::StatBlock(text) => {
            format!("<div class=\"stat-block\">{}</div>\n", render_markdown_text(text))
        }

        Element::BoxedText(text) => {
            format!("<div class=\"boxed-text\">{}</div>\n", escape_html(text))
        }

        Element::Image(image) => {
            render_image(image)
        }

        Element::Table(table) => {
            render_table(table)
        }

        Element::Raw(html) => {
            // Pass through raw HTML
            format!("{}\n", html)
        }
    }
}

fn render_inlines(inlines: &[Inline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) => output.push_str(&escape_html(&apply_typography(text))),
            Inline::Emphasis(inner) => {
                output.push_str(&format!("<em>{}</em>", render_inlines(inner)));
            }
            Inline::Strong(inner) => {
                output.push_str(&format!("<strong>{}</strong>", render_inlines(inner)));
            }
            Inline::Code(code) => {
                output.push_str(&format!("<code>{}</code>", escape_html(code)));
            }
            Inline::Link { text, url } => {
                output.push_str(&format!("<a href=\"{}\">{}</a>", escape_html(url), render_inlines(text)));
            }
            Inline::Image(image) => {
                output.push_str(&render_image(image));
            }
            Inline::SoftBreak => output.push(' '),
            Inline::HardBreak => output.push_str("<br>\n"),
        }
    }
    output
}

fn render_list(ordered: bool, items: &[ListItem]) -> String {
    let tag = if ordered { "ol" } else { "ul" };
    let mut output = format!("<{}>\n", tag);

    for item in items {
        output.push_str("<li>");
        for (i, element) in item.content.iter().enumerate() {
            match element {
                Element::Paragraph(inlines) => {
                    // Don't wrap in <p> for simple list items
                    output.push_str(&render_inlines(inlines));
                }
                Element::List { ordered: nested_ordered, items: nested_items } => {
                    output.push('\n');
                    output.push_str(&render_list(*nested_ordered, nested_items));
                }
                _ => {
                    if i > 0 {
                        output.push('\n');
                    }
                    output.push_str(&render_element(element));
                }
            }
        }
        output.push_str("</li>\n");
    }

    output.push_str(&format!("</{}>\n", tag));
    output
}

fn render_image(image: &Image) -> String {
    let width_style = image.width.as_ref()
        .map(|w| format!(" style=\"width: {}\"", w))
        .unwrap_or_default();
    format!(
        "<img src=\"{}\" alt=\"{}\"{}>\n",
        image.path.display(),
        escape_html(&image.alt),
        width_style
    )
}

fn render_table(table: &Table) -> String {
    let mut output = String::from("<table>\n<thead>\n<tr>\n");

    for (i, header) in table.headers.iter().enumerate() {
        let align = table.alignments.get(i).unwrap_or(&Alignment::None);
        let align_style = match align {
            Alignment::Left => " style=\"text-align: left\"",
            Alignment::Center => " style=\"text-align: center\"",
            Alignment::Right => " style=\"text-align: right\"",
            Alignment::None => "",
        };
        output.push_str(&format!("<th{}>{}</th>\n", align_style, escape_html(header)));
    }

    output.push_str("</tr>\n</thead>\n<tbody>\n");

    for row in &table.rows {
        output.push_str("<tr>\n");
        for (i, cell) in row.iter().enumerate() {
            let align = table.alignments.get(i).unwrap_or(&Alignment::None);
            let align_style = match align {
                Alignment::Left => " style=\"text-align: left\"",
                Alignment::Center => " style=\"text-align: center\"",
                Alignment::Right => " style=\"text-align: right\"",
                Alignment::None => "",
            };
            output.push_str(&format!("<td{}>{}</td>\n", align_style, escape_html(cell)));
        }
        output.push_str("</tr>\n");
    }

    output.push_str("</tbody>\n</table>\n");
    output
}

/// Convert markdown formatting in stat block text to HTML.
///
/// Single newlines are soft (joined with a space, like a markdown paragraph);
/// blank lines emit a hard line break. Heading-prefixed lines (`### `, `#### `)
/// render as bold sub-headings. Lines starting with `- ` or `* ` become bullet
/// list items grouped into a `<ul>`; lines continuing a bullet without a marker
/// are joined to that bullet's text with a space.
fn render_markdown_text(text: &str) -> String {
    let mut blocks: Vec<String> = Vec::new();

    for chunk in text.split("\n\n") {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        let mut block = String::new();
        let mut para_buf: Vec<String> = Vec::new();
        let mut bullets: Vec<String> = Vec::new();
        let mut in_bullets = false;

        let flush_para = |para_buf: &mut Vec<String>, block: &mut String| {
            if !para_buf.is_empty() {
                if !block.is_empty() {
                    block.push_str("<br>\n");
                }
                block.push_str(&convert_inline_markdown(&para_buf.join(" ")));
                para_buf.clear();
            }
        };
        let flush_bullets = |bullets: &mut Vec<String>, block: &mut String| {
            if !bullets.is_empty() {
                block.push_str("<ul>");
                for item in bullets.drain(..) {
                    block.push_str(&format!("<li>{}</li>", convert_inline_markdown(&item)));
                }
                block.push_str("</ul>");
            }
        };

        for line in chunk.lines() {
            let line = line.trim();
            let bullet = line
                .strip_prefix("- ")
                .or_else(|| line.strip_prefix("* "))
                .map(str::to_string);
            let heading = line
                .strip_prefix("#### ")
                .or_else(|| line.strip_prefix("### "))
                .map(str::to_string);

            if let Some(item) = bullet {
                if !in_bullets {
                    flush_para(&mut para_buf, &mut block);
                    in_bullets = true;
                }
                bullets.push(item);
            } else if let Some(heading) = heading {
                flush_para(&mut para_buf, &mut block);
                flush_bullets(&mut bullets, &mut block);
                in_bullets = false;
                if !block.is_empty() {
                    block.push_str("<br>\n");
                }
                block.push_str(&format!("<strong>{}</strong>", escape_html(&heading)));
            } else if in_bullets {
                // Continuation of the previous bullet item.
                if let Some(last) = bullets.last_mut() {
                    last.push(' ');
                    last.push_str(line);
                }
            } else {
                para_buf.push(line.to_string());
            }
        }
        flush_para(&mut para_buf, &mut block);
        flush_bullets(&mut bullets, &mut block);

        if !block.is_empty() {
            blocks.push(block);
        }
    }

    blocks.join("<br>\n")
}

fn convert_inline_markdown(text: &str) -> String {
    let text = apply_typography(text);
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Backslash escape: \X emits X literally (still HTML-escaped if needed).
        if chars[i] == '\\' && i + 1 < chars.len() {
            push_html_char(&mut result, chars[i + 1]);
            i += 2;
            continue;
        }

        // Check for ** (bold)
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_double_star(&chars, i + 2) {
                let inner: String = chars[i + 2..end].iter().collect();
                result.push_str(&format!("<strong>{}</strong>", process_inline_text(&inner)));
                i = end + 2;
                continue;
            }
        }

        // Check for single * (italic): isolated star, no `*` on either side.
        if chars[i] == '*'
            && (i == 0 || chars[i - 1] != '*')
            && (i + 1 >= chars.len() || chars[i + 1] != '*')
        {
            if let Some(end) = find_single_star(&chars, i + 1) {
                let inner: String = chars[i + 1..end].iter().collect();
                result.push_str(&format!("<em>{}</em>", process_inline_text(&inner)));
                i = end + 1;
                continue;
            }
        }

        push_html_char(&mut result, chars[i]);
        i += 1;
    }

    result
}

/// HTML-escape a single character.
fn push_html_char(out: &mut String, c: char) {
    match c {
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '&' => out.push_str("&amp;"),
        '"' => out.push_str("&quot;"),
        _ => out.push(c),
    }
}

/// Process the inside of a bold/italic span: handles backslash escapes and
/// HTML escapes, but does not recurse into nested emphasis.
fn process_inline_text(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            push_html_char(&mut result, chars[i + 1]);
            i += 2;
            continue;
        }
        push_html_char(&mut result, chars[i]);
        i += 1;
    }
    result
}

fn find_double_star(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i + 1 < chars.len() {
        // Skip escaped chars: `\*` is not a delimiter.
        if chars[i] == '\\' && i + 1 < chars.len() {
            i += 2;
            continue;
        }
        if chars[i] == '*' && chars[i + 1] == '*' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Find the next isolated single `*` — i.e. one with no `*` adjacent on
/// either side. This is what closes an italic span; without the prev-side
/// check, we'd happily match the second `*` of a nearby `**` pair.
fn find_single_star(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            i += 2;
            continue;
        }
        let prev_ok = i == 0 || chars[i - 1] != '*';
        let next_ok = i + 1 >= chars.len() || chars[i + 1] != '*';
        if chars[i] == '*' && prev_ok && next_ok {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Typographic substitutions for prose. Run *before* HTML escaping so that
/// `->` is consumed as a unit instead of being split by the `>` escape.
fn apply_typography(text: &str) -> String {
    text.replace("->", "\u{2192}")
}

