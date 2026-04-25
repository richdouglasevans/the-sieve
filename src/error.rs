use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SieveError {
    #[error("Failed to read input file '{path}': {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to write output file '{path}': {source}")]
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse markdown: {0}")]
    ParseMarkdown(String),

    #[error("Failed to compile Typst document: {0}")]
    TypstCompile(String),

    #[error("Failed to render PDF: {0}")]
    PdfRender(String),

    #[error("Invalid stat block format: {0}")]
    InvalidStatBlock(String),

    #[error("Image not found: {0}")]
    ImageNotFound(PathBuf),
}

pub type Result<T> = std::result::Result<T, SieveError>;
