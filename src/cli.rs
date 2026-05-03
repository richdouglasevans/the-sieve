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
    /// Input markdown file
    #[arg(value_name = "INPUT")]
    pub input: PathBuf,

    /// Output PDF file (defaults to input name with .pdf extension)
    #[arg(short, long, value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Output intermediate HTML file instead of PDF
    #[arg(long)]
    pub html_only: bool,
}

impl Args {
    pub fn output_path(&self) -> PathBuf {
        if let Some(ref output) = self.output {
            output.clone()
        } else {
            let mut path = self.input.clone();
            if self.html_only {
                path.set_extension("html");
            } else {
                path.set_extension("pdf");
            }
            path
        }
    }
}

pub fn parse_args() -> Args {
    Args::parse()
}
