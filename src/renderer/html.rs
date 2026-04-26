use std::path::Path;
use std::process::Command;

use crate::ast::{Alignment, Document, Element, Image, Inline, ListItem, Table};
use crate::error::{SieveError, Result};
use crate::licenses::{self, LicenseFragment};

/// Render a document AST to HTML
pub fn render_to_html(document: &Document, _base_path: &Path) -> String {
    let mut output = String::new();
    let mut in_single_column = false;

    // HTML preamble
    output.push_str(&generate_html_preamble());

    output.push_str("<body>\n");
    output.push_str("<div class=\"content two-column\">\n");

    for element in &document.elements {
        match element {
            Element::ColumnLayout(cols) => {
                if *cols == 1 && !in_single_column {
                    // Close two-column div, start single-column
                    output.push_str("</div>\n<div class=\"content single-column\">\n");
                    in_single_column = true;
                } else if *cols == 2 && in_single_column {
                    // Close single-column div, start two-column
                    output.push_str("</div>\n<div class=\"content two-column\">\n");
                    in_single_column = false;
                }
            }
            Element::PageBreak => {
                output.push_str("<div class=\"page-break\"></div>\n");
            }
            Element::License { kind, info } => {
                // Close the current column flow so `break-before: page` on the
                // license wrapper applies cleanly (CSS multicolumn + column-span
                // interacts badly with page-break properties in WeasyPrint).
                let column_class = if in_single_column { "single-column" } else { "two-column" };
                output.push_str("</div>\n");
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
                        LicenseFragment::Heading2(t) => output
                            .push_str(&format!("<h2>{}</h2>\n", escape_html(&t))),
                        LicenseFragment::Heading3(t) => output
                            .push_str(&format!("<h3>{}</h3>\n", escape_html(&t))),
                        LicenseFragment::Paragraph(t) => output
                            .push_str(&format!("<p>{}</p>\n", escape_html(&t))),
                    }
                }
                output.push_str("</div>\n");
                output.push_str("</section>\n");
                output.push_str(&format!("<div class=\"content {}\">\n", column_class));
            }
            Element::Heading { level: 1, text } => {
                // H1 spans all columns
                output.push_str(&format!("<h1>{}</h1>\n", escape_html(text)));
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
@page {
  size: 5.5in 8.5in;
  margin: 0.5in 0.4in;
}

body {
  font-family: Palatino, "Palatino Linotype", Georgia, serif;
  font-size: 9pt;
  line-height: 1.4;
  margin: 0;
  padding: 0;
}

.content.two-column {
  column-count: 2;
  column-gap: 11pt;
  column-fill: balance;
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
  page-break-after: avoid;
}

h2 {
  font-size: 12pt;
  font-weight: bold;
  margin: 1em 0 0.6em 0;
  page-break-after: avoid;
}

h3 {
  font-size: 10pt;
  font-weight: bold;
  margin: 0.8em 0 0.5em 0;
  page-break-after: avoid;
}

h4 {
  font-size: 9pt;
  font-weight: bold;
  margin: 0.6em 0 0.4em 0;
  page-break-after: avoid;
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

/* Nested lists use hollow bullets */
ul ul, ol ul {
  list-style-type: circle;
}

/* Hide bullet for list items that only contain a nested list */
li:has(> ul:first-child):has(> ul:last-child) {
  list-style-type: none;
}
li:has(> ol:first-child):has(> ol:last-child) {
  list-style-type: none;
}

/* Stat block styling */
.stat-block {
  background-color: #e8e8e8;
  padding: 8pt;
  border-radius: 2pt;
  margin: 0.5em 0;
  break-inside: avoid;
}

/* Boxed text (read-aloud) styling */
.boxed-text {
  background-color: #f4f4f0;
  border: 0.5pt solid #999;
  padding: 8pt;
  margin: 0.5em 0;
  font-style: italic;
  break-inside: avoid;
}

/* Code blocks */
pre {
  font-family: monospace;
  font-size: 8pt;
  background-color: #f5f5f5;
  padding: 8pt;
  overflow-x: auto;
  break-inside: avoid;
}

code {
  font-family: monospace;
  font-size: 8pt;
  background-color: #f5f5f5;
  padding: 1pt 3pt;
}

/* Tables */
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

/* Blockquotes */
blockquote {
  margin: 0.5em 0 0.5em 1em;
  padding-left: 0.5em;
  border-left: 2pt solid #999;
  font-style: italic;
}

/* Links */
a {
  color: #333;
  text-decoration: underline;
}

/* Images */
img {
  max-width: 100%;
  height: auto;
}

/* Thematic break */
hr {
  border: none;
  border-top: 1pt solid #999;
  margin: 1em 0;
}

/* Page break */
.page-break {
  break-after: page;
}

/* License page */
.license-section {
  break-before: page;
  page-break-before: always;
}

.license-title {
  text-align: center;
  font-size: 13pt;
  font-weight: bold;
  margin: 0.5em 0 0.5em 0;
  page-break-after: avoid;
}

.license-attribution {
  font-size: 9pt;
  font-weight: bold;
  margin: 0.4em 0;
}

.license-changes {
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
  page-break-after: avoid;
}

.license h3 {
  font-size: 7pt;
  font-weight: bold;
  margin: 0.4em 0 0.2em 0;
  page-break-after: avoid;
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
            Inline::Text(text) => output.push_str(&escape_html(text)),
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

/// Convert markdown formatting in stat block text to HTML
fn render_markdown_text(text: &str) -> String {
    let mut result = String::new();

    for line in text.lines() {
        let line = line.trim();

        // Handle headings
        if let Some(heading) = line.strip_prefix("#### ") {
            result.push_str(&format!("<strong>{}</strong><br>\n", escape_html(heading)));
            continue;
        } else if let Some(heading) = line.strip_prefix("### ") {
            result.push_str(&format!("<strong>{}</strong><br>\n", escape_html(heading)));
            continue;
        }

        // Convert **bold** and *italic*
        let converted = convert_inline_markdown(line);
        result.push_str(&converted);
        result.push_str("<br>\n");
    }

    // Remove trailing <br>
    if result.ends_with("<br>\n") {
        result.truncate(result.len() - 5);
    }

    result
}

fn convert_inline_markdown(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Check for ** (bold)
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_double_star(&chars, i + 2) {
                let inner: String = chars[i + 2..end].iter().collect();
                result.push_str(&format!("<strong>{}</strong>", escape_html(&inner)));
                i = end + 2;
                continue;
            }
        }

        // Check for single * (italic)
        if chars[i] == '*' && (i == 0 || chars[i - 1] != '*') && (i + 1 >= chars.len() || chars[i + 1] != '*') {
            if let Some(end) = find_single_star(&chars, i + 1) {
                let inner: String = chars[i + 1..end].iter().collect();
                result.push_str(&format!("<em>{}</em>", escape_html(&inner)));
                i = end + 1;
                continue;
            }
        }

        // Regular character - escape HTML
        let c = chars[i];
        match c {
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '&' => result.push_str("&amp;"),
            '"' => result.push_str("&quot;"),
            _ => result.push(c),
        }
        i += 1;
    }

    result
}

fn find_double_star(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i + 1 < chars.len() {
        if chars[i] == '*' && chars[i + 1] == '*' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_single_star(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i < chars.len() {
        if chars[i] == '*' && (i + 1 >= chars.len() || chars[i + 1] != '*') {
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

/// Compile HTML to PDF using WeasyPrint
pub fn compile_to_pdf(html: &str, base_path: &Path) -> Result<Vec<u8>> {
    // Write HTML to a temp file (use absolute paths)
    let abs_base = base_path.canonicalize().unwrap_or_else(|_| base_path.to_path_buf());
    let html_path = abs_base.join(".the_sieve_temp.html");
    let pdf_path = abs_base.join(".the_sieve_temp.pdf");

    std::fs::write(&html_path, html)
        .map_err(|e| SieveError::WriteFile {
            path: html_path.clone(),
            source: e,
        })?;

    // Call WeasyPrint
    let output = Command::new("weasyprint")
        .arg(&html_path)
        .arg(&pdf_path)
        .current_dir(&abs_base)
        .output()
        .map_err(|e| SieveError::PdfRender(format!("Failed to run weasyprint: {}", e)))?;

    // Clean up temp HTML
    let _ = std::fs::remove_file(&html_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_file(&pdf_path);
        return Err(SieveError::PdfRender(format!("WeasyPrint failed: {}", stderr)));
    }

    // Read the PDF
    let pdf_data = std::fs::read(&pdf_path)
        .map_err(|e| SieveError::ReadFile {
            path: pdf_path.clone(),
            source: e,
        })?;

    // Clean up temp PDF
    let _ = std::fs::remove_file(&pdf_path);

    Ok(pdf_data)
}

/// Full pipeline: AST -> HTML -> PDF
pub fn render_to_pdf(document: &Document, base_path: &Path) -> Result<Vec<u8>> {
    let html = render_to_html(document, base_path);
    compile_to_pdf(&html, base_path)
}
