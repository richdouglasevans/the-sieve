# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A Rust CLI that converts TTRPG-flavored markdown into half-letter (5.5" × 8.5") PDFs sized for booklet printing. The binary is `the-sieve`; the library crate name is `the_sieve`.

## Commands

```sh
cargo build --release            # binary at target/release/the-sieve
cargo test                       # all unit tests (inline #[test] modules; no tests/ dir)
cargo test -p the-sieve <name>   # run a single test by name substring
cargo run -- <INPUT.md>          # convert a markdown file to PDF
```

Common CLI flags (see `src/cli.rs`): `-o OUTPUT`, `-v`, `--html-only` (emit intermediate HTML), `--typst-only` (emit intermediate Typst).

The default PDF pipeline shells out to `weasyprint`, which must be on `PATH` (`brew install weasyprint` on macOS). Without it, only `--html-only` and `--typst-only` work.

## Architecture

The pipeline is **markdown → AST → renderer → PDF**, with two interchangeable renderers:

1. **HTML renderer (default)** — `src/renderer/html.rs`. Emits HTML, then spawns WeasyPrint as a subprocess to produce the PDF. Chosen as the default because WeasyPrint balances multi-column text well.
2. **Typst renderer** — `src/renderer/typst.rs`. Emits Typst source and compiles it in-process via the embedded `typst` crate (no external dependency). Exposed via `--typst-only` for inspection; `compile_to_pdf` here uses a custom `SieveWorld` implementing Typst's `World` trait to load system fonts and resolve files.

Both renderers consume the same intermediate AST defined in `src/ast.rs` (`Document` → `Element` enum). The library entry points (`convert_markdown_to_pdf`, `convert_markdown_to_html`, `convert_markdown_to_typst`, `parse_markdown`) live in `src/lib.rs`; `src/main.rs` is a thin wrapper that picks an output path based on flags.

### Parser

`src/parser/markdown.rs` wraps `pulldown-cmark` and walks events into the AST. TTRPG-specific extensions live in `src/parser/extensions.rs`:

- `<!-- pagebreak -->` HTML comments → `Element::PageBreak`
- Fenced code blocks with language tags `statblock` / `stat-block` / `monster` → `Element::StatBlock` (shaded box)
- Fenced code blocks with `boxed` / `read-aloud` / `readaloud` → `Element::BoxedText` (read-aloud styling)
- Fenced code blocks with `columns` → multi-column layout, with `---` as a column separator

When adding a new extension, the typical change touches all three layers: detect it in `extensions.rs`, add an `Element` variant in `ast.rs`, and render it in **both** `renderer/html.rs` and `renderer/typst.rs` to keep the two pipelines in parity.

### Templates

`templates/default.typ` is the Typst styling baseline (page geometry, headings, stat-block / boxed-text helpers). The Typst renderer embeds an equivalent preamble inline in `generate_preamble()` rather than loading from disk; if you change `templates/default.typ`, the preamble in `renderer/typst.rs` likely needs the same change.

### Half-letter output

Page geometry (5.5" × 8.5", two-column, narrow margins) is hardcoded in both the HTML CSS and the Typst preamble — the format is the project's identity, not a parameter.
