use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::process::ExitCode;

use the_sieve::cli::{parse_args, Args, OutputDest};
use the_sieve::error::SieveError;
use the_sieve::{convert_markdown_to_html, convert_markdown_to_pdf};

fn main() -> ExitCode {
    let args = parse_args();

    if let Err(e) = run(&args) {
        eprintln!("Error: {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run(args: &Args) -> Result<(), SieveError> {
    let (input_content, base_path) = match &args.input {
        Some(path) => {
            if args.verbose {
                eprintln!("Reading: {}", path.display());
            }

            let content =
                fs::read_to_string(path).map_err(|e| SieveError::ReadFile {
                    path: path.clone(),
                    source: e,
                })?;

            // `Path::parent` returns `Some("")` for a bare filename like `FOO.md`;
            // fall back to CWD so image-path resolution doesn't break.
            let base = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

            (content, base)
        }
        None => {
            if io::stdin().is_terminal() {
                return Err(SieveError::NoInput);
            }

            if args.verbose {
                eprintln!("Reading from stdin...");
            }

            let mut content = String::new();
            io::stdin()
                .lock()
                .read_to_string(&mut content)
                .map_err(|e| SieveError::ReadFile {
                    path: "<stdin>".into(),
                    source: e,
                })?;

            let base = std::env::current_dir().unwrap_or_default();
            (content, base)
        }
    };

    let output_dest = args.output_dest();

    if args.html_only {
        let html_source = convert_markdown_to_html(&input_content, &base_path)?;

        match output_dest {
            OutputDest::File(path) => {
                if args.verbose {
                    eprintln!("Writing HTML: {}", path.display());
                }
                fs::write(&path, &html_source).map_err(|e| SieveError::WriteFile {
                    path: path.clone(),
                    source: e,
                })?;
                eprintln!("Created: {}", path.display());
            }
            OutputDest::Stdout => {
                if args.verbose {
                    eprintln!("Writing HTML to stdout...");
                }
                io::stdout()
                    .write_all(html_source.as_bytes())
                    .map_err(SieveError::WriteStdout)?;
            }
        }
    } else {
        if args.verbose {
            eprintln!("Generating PDF...");
        }

        let pdf_data = convert_markdown_to_pdf(&input_content, &base_path)?;

        match output_dest {
            OutputDest::File(path) => {
                if args.verbose {
                    eprintln!("Writing PDF: {}", path.display());
                }
                fs::write(&path, &pdf_data).map_err(|e| SieveError::WriteFile {
                    path: path.clone(),
                    source: e,
                })?;
                eprintln!("Created: {}", path.display());
            }
            OutputDest::Stdout => {
                if args.verbose {
                    eprintln!("Writing PDF to stdout...");
                }
                io::stdout()
                    .write_all(&pdf_data)
                    .map_err(SieveError::WriteStdout)?;
            }
        }
    }

    Ok(())
}
