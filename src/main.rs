use std::fs;
use std::process::ExitCode;

use the_sieve::cli::{parse_args, Args};
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
    let input_content = fs::read_to_string(&args.input).map_err(|e| SieveError::ReadFile {
        path: args.input.clone(),
        source: e,
    })?;

    if args.verbose {
        eprintln!("Reading: {}", args.input.display());
    }

    // `Path::parent` returns `Some("")` for a bare filename like `FOO.md`;
    // fall back to CWD so image-path resolution doesn't break.
    let base_path = args
        .input
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let output_path = args.output_path();

    if args.html_only {
        let html_source = convert_markdown_to_html(&input_content, &base_path)?;

        if args.verbose {
            eprintln!("Writing HTML: {}", output_path.display());
        }

        fs::write(&output_path, &html_source).map_err(|e| SieveError::WriteFile {
            path: output_path.clone(),
            source: e,
        })?;

        eprintln!("Created: {}", output_path.display());
    } else {
        if args.verbose {
            eprintln!("Generating PDF...");
        }

        let pdf_data = convert_markdown_to_pdf(&input_content, &base_path)?;

        if args.verbose {
            eprintln!("Writing PDF: {}", output_path.display());
        }

        fs::write(&output_path, &pdf_data).map_err(|e| SieveError::WriteFile {
            path: output_path.clone(),
            source: e,
        })?;

        eprintln!("Created: {}", output_path.display());
    }

    Ok(())
}
