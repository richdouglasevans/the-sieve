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

    #[error("Failed to render PDF: {0}")]
    PdfRender(String),

    #[error("Invalid stat block format: {0}")]
    InvalidStatBlock(String),

    #[error("Image not found: {0}")]
    ImageNotFound(PathBuf),

    #[error("no input provided.\n\nUsage:\n  the-sieve <INPUT>            read from file\n  cat file.md | the-sieve      read from stdin\n\nFor more information, try '--help'.")]
    NoInput,

    #[error("Failed to write to stdout: {0}")]
    WriteStdout(std::io::Error),
}

pub type Result<T> = std::result::Result<T, SieveError>;
