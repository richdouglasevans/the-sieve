use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "the-sieve",
    author = "The Sieve",
    version,
    about = "Convert TTRPG markdown to half-letter PDFs for booklet printing",
    long_about = "The Sieve converts markdown documents with TTRPG-specific extensions
(stat blocks, boxed read-aloud text, layout switching) into professionally typeset
PDFs sized for half-letter (5.5\" x 8.5\") booklet printing."
)]
pub struct Args {
    /// Input markdown file (omit to read from stdin)
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    /// Output file (defaults to input name with .pdf/.html extension, or stdout when reading from stdin)
    #[arg(short, long, value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Output intermediate HTML file instead of PDF
    #[arg(long)]
    pub html_only: bool,
}

pub enum OutputDest {
    File(PathBuf),
    Stdout,
}

impl Args {
    pub fn output_dest(&self) -> OutputDest {
        if let Some(ref o) = self.output {
            return OutputDest::File(o.clone());
        }
        if let Some(ref i) = self.input {
            let mut path = i.clone();
            path.set_extension(if self.html_only { "html" } else { "pdf" });
            return OutputDest::File(path);
        }
        OutputDest::Stdout
    }
}

pub fn parse_args() -> Args {
    Args::parse()
}
