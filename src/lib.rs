//! The Sieve - TTRPG Markdown to PDF converter
//!
//! This library provides functionality to convert markdown documents with
//! TTRPG-specific extensions into professionally typeset PDFs.
//!
//! # Features
//!
//! - **Stat blocks**: D&D 5e-style monster/NPC stat blocks
//! - **Boxed text**: Read-aloud text with distinct styling
//! - **Layout switching**: `<!-- 1-column -->` / `<!-- 2-column -->` directives
//! - **Page breaks**: Explicit page break control
//! - **Half-letter format**: 5.5" x 8.5" for booklet printing
//!
//! # Example
//!
//! ```rust,ignore
//! use the_sieve::convert_markdown_to_pdf;
//! use std::path::Path;
//!
//! let markdown = "# Hello\n\nA paragraph.";
//! let pdf_data = convert_markdown_to_pdf(markdown, Path::new(".")).unwrap();
//! ```

pub mod ast;
pub mod cli;
pub mod error;
pub mod licenses;
pub mod parser;
pub mod renderer;

use std::path::Path;

pub use ast::{Document, Element};
pub use error::{Result, SieveError};

/// Convert markdown content to PDF bytes via the native krilla+parley pipeline.
///
/// # Arguments
///
/// * `markdown` - The markdown content to convert
/// * `base_path` - Base path for resolving relative image paths
pub fn convert_markdown_to_pdf(markdown: &str, base_path: &Path) -> Result<Vec<u8>> {
    let document = parser::parse(markdown)?;
    renderer::pdf::render(&document, base_path).map_err(|e| SieveError::PdfRender(e.to_string()))
}

/// Convert markdown content to HTML.
///
/// Useful for debugging the parsed document structure or manually editing the
/// intermediate output. Note that the HTML path is no longer the production
/// PDF route — `convert_markdown_to_pdf` renders directly via krilla+parley.
pub fn convert_markdown_to_html(markdown: &str, base_path: &Path) -> Result<String> {
    let document = parser::parse(markdown)?;
    Ok(renderer::render_to_html(&document, base_path))
}

/// Parse markdown into the intermediate AST.
pub fn parse_markdown(markdown: &str) -> Result<Document> {
    parser::parse(markdown)
}
