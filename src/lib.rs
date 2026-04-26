//! The Sieve - TTRPG Markdown to PDF converter
//!
//! This library provides functionality to convert markdown documents with
//! TTRPG-specific extensions into professionally typeset PDFs.
//!
//! # Features
//!
//! - **Stat blocks**: D&D 5e-style monster/NPC stat blocks
//! - **Boxed text**: Read-aloud text with distinct styling
//! - **Columns**: Multi-column layouts
//! - **Page breaks**: Explicit page break control
//! - **Half-letter format**: 5.5" x 8.5" for booklet printing
//!
//! # Example
//!
//! ```rust,ignore
//! use the_sieve::{convert_markdown_to_pdf, Options};
//! use std::path::Path;
//!
//! let markdown = r#"
//! # The Goblin Cave
//!
//! ```boxed
//! The cavern mouth yawns before you, darkness within.
//! ```
//!
//! ## Encounter
//!
//! ```statblock
//! name: Goblin
//! ac: 15 (leather armor, shield)
//! hp: 7 (2d6)
//! speed: 30 ft.
//! str: 8
//! dex: 14
//! con: 10
//! int: 10
//! wis: 8
//! cha: 8
//! ```
//! "#;
//!
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

/// Convert markdown content to PDF bytes
///
/// # Arguments
///
/// * `markdown` - The markdown content to convert
/// * `base_path` - Base path for resolving relative image paths
///
/// # Returns
///
/// PDF file contents as a byte vector
pub fn convert_markdown_to_pdf(markdown: &str, base_path: &Path) -> Result<Vec<u8>> {
    let document = parser::parse(markdown)?;
    renderer::render_to_pdf(&document, base_path)
}

/// Convert markdown content to HTML
///
/// This is useful for debugging or manual HTML editing.
///
/// # Arguments
///
/// * `markdown` - The markdown content to convert
/// * `base_path` - Base path for resolving relative image paths
///
/// # Returns
///
/// HTML source code as a string
pub fn convert_markdown_to_html(markdown: &str, base_path: &Path) -> Result<String> {
    let document = parser::parse(markdown)?;
    Ok(renderer::render_to_html(&document, base_path))
}

/// Convert markdown content to Typst source
///
/// This is useful for debugging or manual Typst editing.
///
/// # Arguments
///
/// * `markdown` - The markdown content to convert
/// * `base_path` - Base path for resolving relative image paths
///
/// # Returns
///
/// Typst source code as a string
pub fn convert_markdown_to_typst(markdown: &str, base_path: &Path) -> Result<String> {
    let document = parser::parse(markdown)?;
    Ok(renderer::render_to_typst(&document, base_path))
}

/// Parse markdown into the intermediate AST
///
/// Useful for inspection or custom rendering.
pub fn parse_markdown(markdown: &str) -> Result<Document> {
    parser::parse(markdown)
}

/// Compile HTML directly to PDF using WeasyPrint
///
/// Use this for HTML files that don't need markdown conversion.
///
/// # Arguments
///
/// * `html` - The HTML source code
/// * `base_path` - Base path for resolving relative paths (images, etc.)
///
/// # Returns
///
/// PDF file contents as a byte vector
pub fn compile_html_to_pdf(html: &str, base_path: &Path) -> Result<Vec<u8>> {
    renderer::compile_to_pdf(html, base_path)
}
