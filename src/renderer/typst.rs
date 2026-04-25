use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ecow::EcoVec;
use typst::diag::{FileError, FileResult, Severity, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_pdf::PdfOptions;

use crate::ast::{Alignment, Document, Element, Image, Inline, ListItem, Table};
use crate::error::{SieveError, Result};

/// Render a document AST to Typst markup
pub fn render_to_typst(document: &Document, base_path: &Path) -> String {
    let mut output = String::new();
    let mut in_columns = false;

    // Document preamble with page setup
    output.push_str(&generate_preamble());

    let start_cols = "#columns(2, gutter: 11pt)[\n";
    let end_cols = "]\n";

    // Render each element
    for element in &document.elements {
        match element {
            Element::ColumnLayout(cols) => {
                if *cols == 1 && in_columns {
                    // Close columns for single-column mode
                    output.push_str(end_cols);
                    in_columns = false;
                } else if *cols == 2 && !in_columns {
                    // Start columns
                    output.push_str(start_cols);
                    in_columns = true;
                }
            }
            Element::PageBreak => {
                if in_columns {
                    output.push_str(end_cols);
                    output.push_str("#pagebreak()\n");
                    output.push_str(start_cols);
                } else {
                    output.push_str("#pagebreak()\n");
                }
            }
            Element::Heading { level: 1, text } => {
                // H1: close columns, render centered, reopen
                if in_columns {
                    output.push_str(end_cols);
                }
                output.push_str(&format!(
                    "#align(center)[#text(size: 18pt, weight: \"bold\")[{}]]\n",
                    escape_typst(text)
                ));
                if in_columns {
                    output.push_str(start_cols);
                }
            }
            _ => {
                // Auto-start columns for content if not already in columns
                if !in_columns {
                    output.push_str(start_cols);
                    in_columns = true;
                }
                output.push_str(&render_element(element, base_path));
                output.push('\n');
            }
        }
    }

    // Close any open columns
    if in_columns {
        output.push_str(end_cols);
    }

    output
}

fn generate_preamble() -> String {
    r##"// The Sieve TTRPG Document
#set page(
  width: 5.5in,
  height: 8.5in,
  margin: (top: 0.5in, bottom: 0.5in, left: 0.4in, right: 0.4in),
)

#set text(
  font: ("Palatino", "Palatino Linotype", "Georgia", "serif"),
  size: 9pt,
  hyphenate: true,
)

#set par(
  justify: false,
  leading: 0.6em,
  first-line-indent: 0pt,
)

#set heading(numbering: none)

#show heading.where(level: 1): it => {
  set align(center)
  set text(size: 18pt, weight: "bold")
  block(above: 0.5em, below: 0.8em)[#it.body]
}

#show heading.where(level: 2): it => {
  set text(size: 12pt, weight: "bold")
  block(above: 1em, below: 0.6em)[#it.body]
}

#show heading.where(level: 3): it => {
  set text(size: 10pt, weight: "bold")
  block(above: 0.8em, below: 0.5em)[#it.body]
}

#show heading.where(level: 4): it => {
  set text(size: 9pt, weight: "bold")
  block(above: 0.6em, below: 0.4em)[#it.body]
}

// OSR-style stat block - simple shaded box
#let stat-block(content) = {
  block(
    width: 100%,
    fill: rgb("#e8e8e8"),
    inset: 8pt,
    radius: 2pt,
    breakable: false,
  )[
    #content
  ]
}

// Boxed text (read-aloud) styling
#let boxed-text(content) = {
  block(
    width: 100%,
    fill: rgb("#f4f4f0"),
    stroke: 0.5pt + rgb("#999"),
    inset: 8pt,
    radius: 0pt,
  )[
    #set text(style: "italic")
    #content
  ]
}

"##
    .to_string()
}

fn render_element(element: &Element, base_path: &Path) -> String {
    match element {
        Element::Heading { level, text } => {
            let hashes = "=".repeat(*level as usize);
            format!("{} {}\n", hashes, escape_typst(text))
        }

        Element::Paragraph(inlines) => {
            let content = render_inlines(inlines);
            format!("{}\n", content)
        }

        Element::CodeBlock { language, code } => {
            let lang = language.as_deref().unwrap_or("");
            format!("```{}\n{}```\n", lang, code)
        }

        Element::BlockQuote(elements) => {
            let content: String = elements
                .iter()
                .map(|e| render_element(e, base_path))
                .collect();
            // Indent each line with >
            let quoted: String = content
                .lines()
                .map(|line| format!("> {}", line))
                .collect::<Vec<_>>()
                .join("\n");
            format!("#quote[{}]\n", quoted)
        }

        Element::List { ordered, items } => render_list(*ordered, items, base_path, 0),

        Element::ThematicBreak => "#line(length: 100%)\n".to_string(),

        Element::PageBreak => {
            // Handled in render_to_typst main loop
            String::new()
        }

        Element::ColumnLayout(_) => {
            // Handled in render_to_typst main loop
            String::new()
        }

        Element::StatBlock(text) => {
            format!("#stat-block[{}]\n", convert_markdown_to_typst_content(text))
        }

        Element::BoxedText(text) => {
            format!("#boxed-text[{}]\n", convert_markdown_to_typst_content(text))
        }


        Element::Image(image) => render_image(image, base_path),

        Element::Table(table) => render_table(table),

        Element::Raw(html) => {
            // Skip raw HTML in Typst output
            format!("// Raw HTML: {}\n", html.trim())
        }
    }
}

fn render_inlines(inlines: &[Inline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) => output.push_str(&escape_typst(text)),
            Inline::Emphasis(inner) => {
                output.push_str(&format!("_{}_", render_inlines(inner)));
            }
            Inline::Strong(inner) => {
                output.push_str(&format!("*{}*", render_inlines(inner)));
            }
            Inline::Code(code) => {
                output.push_str(&format!("`{}`", code));
            }
            Inline::Link { text, url } => {
                let text_str = render_inlines(text);
                output.push_str(&format!("#link(\"{}\")[{}]", url, text_str));
            }
            Inline::Image(image) => {
                // Inline images - just use the path for now
                output.push_str(&format!(
                    "#image(\"{}\")",
                    image.path.display()
                ));
            }
            Inline::SoftBreak => output.push(' '),
            Inline::HardBreak => output.push_str("\\\n"),
        }
    }
    output
}

fn render_list(ordered: bool, items: &[ListItem], base_path: &Path, depth: usize) -> String {
    let mut output = String::new();
    let indent = "  ".repeat(depth);

    for (i, item) in items.iter().enumerate() {
        let marker = if ordered {
            format!("{}. ", i + 1)
        } else {
            "- ".to_string()
        };

        // Separate nested lists from other content
        let mut text_parts: Vec<String> = Vec::new();
        let mut nested_lists: Vec<String> = Vec::new();

        for element in &item.content {
            match element {
                Element::List { ordered: nested_ordered, items: nested_items } => {
                    nested_lists.push(render_list(*nested_ordered, nested_items, base_path, depth + 1));
                }
                _ => {
                    let rendered = render_element(element, base_path);
                    let trimmed = rendered.trim();
                    if !trimmed.is_empty() {
                        text_parts.push(trimmed.to_string());
                    }
                }
            }
        }

        // Combine text parts on one line
        let text_content = text_parts.join(" ").replace('\n', " ");
        output.push_str(&format!("{}{}{}\n", indent, marker, text_content));

        // Add nested lists with proper indentation
        for nested in nested_lists {
            output.push_str(&nested);
        }
    }

    output
}

fn render_image(image: &Image, base_path: &Path) -> String {
    let path = if image.path.is_absolute() {
        image.path.clone()
    } else {
        base_path.join(&image.path)
    };

    let width = image
        .width
        .as_ref()
        .map(|w| format!(", width: {}", w))
        .unwrap_or_default();

    format!("#image(\"{}\"{})\n", path.display(), width)
}

fn render_table(table: &Table) -> String {
    let num_cols = table.headers.len();
    let mut output = format!("#table(\n  columns: {},\n  inset: 6pt,\n", num_cols);

    // Add alignment
    let aligns: Vec<&str> = table
        .alignments
        .iter()
        .map(|a| match a {
            Alignment::Left => "left",
            Alignment::Center => "center",
            Alignment::Right => "right",
            Alignment::None => "auto",
        })
        .collect();
    output.push_str(&format!("  align: ({}),\n", aligns.join(", ")));

    // Headers - all in one table.header call
    let headers: Vec<String> = table
        .headers
        .iter()
        .map(|h| format!("[*{}*]", escape_typst(h)))
        .collect();
    output.push_str(&format!("  table.header({}),\n", headers.join(", ")));

    // Rows
    for row in &table.rows {
        for cell in row {
            output.push_str(&format!("  [{}],\n", escape_typst(cell)));
        }
    }

    output.push_str(")\n");
    output
}

fn escape_typst(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('#', "\\#")
        .replace('$', "\\$")
        .replace('@', "\\@")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('*', "\\*")
        .replace('_', "\\_")
}

/// Convert markdown content to Typst, handling bold, italic, and headings
fn convert_markdown_to_typst_content(text: &str) -> String {
    let mut result = String::new();

    for line in text.lines() {
        let trimmed = line.trim_start();

        // Handle markdown headings (#### -> bold text)
        if let Some(heading_text) = trimmed.strip_prefix("#### ") {
            result.push_str(&format!("*{}*", escape_typst_minimal(heading_text)));
            result.push_str("\\\n");
            continue;
        } else if let Some(heading_text) = trimmed.strip_prefix("### ") {
            result.push_str(&format!("*{}*", escape_typst_minimal(heading_text)));
            result.push_str("\\\n");
            continue;
        } else if let Some(heading_text) = trimmed.strip_prefix("## ") {
            result.push_str(&format!("*{}*", escape_typst_minimal(heading_text)));
            result.push_str("\\\n");
            continue;
        }

        // Convert markdown formatting in the line
        let converted = convert_markdown_formatting(line);
        result.push_str(&converted);
        result.push_str("\\\n");
    }

    // Remove trailing line break
    if result.ends_with("\\\n") {
        result.truncate(result.len() - 2);
    }

    result
}

/// Convert markdown bold/italic to Typst formatting
fn convert_markdown_formatting(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Check for ** (bold)
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            // Find closing **
            if let Some(end) = find_closing_double_star(&chars, i + 2) {
                result.push('*');
                let inner: String = chars[i + 2..end].iter().collect();
                result.push_str(&escape_typst_minimal(&inner));
                result.push('*');
                i = end + 2;
                continue;
            }
        }

        // Check for single * (italic) - but not if it's part of **
        if chars[i] == '*' && (i == 0 || chars[i - 1] != '*') && (i + 1 >= chars.len() || chars[i + 1] != '*') {
            // Find closing *
            if let Some(end) = find_closing_single_star(&chars, i + 1) {
                result.push('_');
                let inner: String = chars[i + 1..end].iter().collect();
                result.push_str(&escape_typst_minimal(&inner));
                result.push('_');
                i = end + 1;
                continue;
            }
        }

        // Escape special Typst characters
        let c = chars[i];
        match c {
            '#' => result.push_str("\\#"),
            '$' => result.push_str("\\$"),
            '@' => result.push_str("\\@"),
            '<' => result.push_str("\\<"),
            '>' => result.push_str("\\>"),
            '[' => result.push_str("\\["),
            ']' => result.push_str("\\]"),
            '\\' => result.push_str("\\\\"),
            _ => result.push(c),
        }
        i += 1;
    }

    result
}

fn find_closing_double_star(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i + 1 < chars.len() {
        if chars[i] == '*' && chars[i + 1] == '*' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_closing_single_star(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i < chars.len() {
        if chars[i] == '*' && (i + 1 >= chars.len() || chars[i + 1] != '*') {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Escape Typst special chars but NOT * and _ (for use inside already-converted formatting)
fn escape_typst_minimal(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('#', "\\#")
        .replace('$', "\\$")
        .replace('@', "\\@")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

/// A minimal World implementation for Typst compilation
pub struct SieveWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    source: Source,
    files: HashMap<FileId, Bytes>,
    base_path: PathBuf,
}

impl SieveWorld {
    pub fn new(source_content: String, base_path: PathBuf) -> Self {
        let fonts = Self::load_fonts();
        let book = FontBook::from_fonts(&fonts);

        let source = Source::detached(source_content);

        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            source,
            files: HashMap::new(),
            base_path,
        }
    }

    fn load_fonts() -> Vec<Font> {
        let mut fonts = Vec::new();

        // Try to load system fonts from common locations
        let font_paths = [
            "/System/Library/Fonts",
            "/Library/Fonts",
            "/usr/share/fonts",
            "/usr/local/share/fonts",
        ];

        for font_path in font_paths {
            if let Ok(entries) = std::fs::read_dir(font_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| {
                        e == "ttf" || e == "otf" || e == "ttc"
                    }) {
                        if let Ok(data) = std::fs::read(&path) {
                            let buffer = Bytes::new(data);
                            for font in Font::iter(buffer) {
                                fonts.push(font);
                            }
                        }
                    }
                }
            }
        }

        // Also check user font directories
        if let Some(home) = dirs_next().next() {
            let user_fonts = home.join("Library/Fonts");
            if let Ok(entries) = std::fs::read_dir(&user_fonts) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| {
                        e == "ttf" || e == "otf" || e == "ttc"
                    }) {
                        if let Ok(data) = std::fs::read(&path) {
                            let buffer = Bytes::new(data);
                            for font in Font::iter(buffer) {
                                fonts.push(font);
                            }
                        }
                    }
                }
            }
        }

        fonts
    }
}

fn dirs_next() -> impl Iterator<Item = PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .into_iter()
}

impl World for SieveWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        // Try to load from our cache first
        if let Some(bytes) = self.files.get(&id) {
            return Ok(bytes.clone());
        }

        // Try to load from disk
        let path = self.base_path.join(id.vpath().as_rootless_path());
        match std::fs::read(&path) {
            Ok(data) => Ok(Bytes::new(data)),
            Err(_) => Err(FileError::NotFound(path)),
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        Some(Datetime::from_ymd(2024, 1, 1).unwrap())
    }
}

/// Compile Typst source to PDF
pub fn compile_to_pdf(typst_source: &str, base_path: &Path) -> Result<Vec<u8>> {
    let world = SieveWorld::new(typst_source.to_string(), base_path.to_path_buf());

    let result = typst::compile(&world);

    match result.output {
        Ok(document) => {
            let options = PdfOptions::default();
            match typst_pdf::pdf(&document, &options) {
                Ok(pdf) => Ok(pdf),
                Err(errors) => {
                    let msg = format_errors(&errors);
                    Err(SieveError::PdfRender(msg))
                }
            }
        }
        Err(errors) => {
            let msg = format_errors(&errors);
            Err(SieveError::TypstCompile(msg))
        }
    }
}

fn format_errors(errors: &EcoVec<SourceDiagnostic>) -> String {
    errors
        .iter()
        .filter(|e| e.severity == Severity::Error)
        .map(|e| e.message.to_string())
        .collect::<Vec<_>>()
        .join("; ")
}

/// Full pipeline: AST -> Typst -> PDF
pub fn render_to_pdf(document: &Document, base_path: &Path) -> Result<Vec<u8>> {
    let typst_source = render_to_typst(document, base_path);
    compile_to_pdf(&typst_source, base_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preamble_generation() {
        let preamble = generate_preamble();
        assert!(preamble.contains("5.5in"));
        assert!(preamble.contains("8.5in"));
    }

    #[test]
    fn test_heading_render() {
        let element = Element::Heading {
            level: 1,
            text: "Test Heading".to_string(),
        };
        let output = render_element(&element, Path::new("."));
        assert!(output.contains("= Test Heading"));
    }
}
