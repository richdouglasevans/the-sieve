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

CLI flags (see `src/cli.rs`): `-o OUTPUT`, `-v`, `--html-only` (emit intermediate HTML for debugging).

The build is fully self-contained — fonts are embedded via `include_bytes!`, there are no runtime dependencies.

## Architecture

The pipeline is **markdown → AST → PDF**:

1. `src/parser/markdown.rs` walks `pulldown-cmark` events into an intermediate `Document` (see `src/ast.rs`).
2. `src/renderer/pdf.rs` lays out the document directly to PDF via [krilla](https://crates.io/crates/krilla) (PDF output) and [parley](https://crates.io/crates/parley) (paragraph layout / line breaking / font fallback).

`src/renderer/html.rs` is retained only for `--html-only` (a debug-style intermediate output); it is not part of the PDF path.

Library entry points (`convert_markdown_to_pdf`, `convert_markdown_to_html`, `parse_markdown`) live in `src/lib.rs`; `src/main.rs` is a thin wrapper that picks an output path based on flags.

### Native PDF renderer

`src/renderer/pdf.rs` implements a paginated multicolumn layout engine:

- **Tracks**: document elements are grouped into runs of single-column or two-column content. `<!-- 1-column -->` / `<!-- 2-column -->` directives split tracks; `<!-- pagebreak -->` terminates a track with `page_break_after = true`.
- **Two-column with balance**: when a 2-col track fits in remaining vertical space, it balances (split point chosen so both columns are roughly equal). On overflow, columns greedy-fill with paragraph splitting along line boundaries.
- **H1 in 2-col mode** spans both columns as a centered 1-col band between flows.
- **License directive** flushes the current track with a forced page break, then emits a 1-col header (title, attribution, changes) and a 2-col body of license fragments at 6.5pt.
- **Splittable blocks**: paragraphs and license body paragraphs split line-by-line when they don't fit; headings, tables, stat blocks, boxed text, code blocks, images stay atomic.

### Parser extensions

TTRPG-specific syntax lives in `src/parser/extensions.rs`:

- `<!-- pagebreak -->` HTML comments → `Element::PageBreak`
- `<!-- license: ogl-1.0a -->` and `<!-- license: cc-by-sa-4.0 -->` → `Element::License`. CC-BY-SA accepts optional `attribution="..."` and `changes="..."` parameters that render above the body.
- Fenced code blocks with language tags `statblock` / `stat-block` / `monster` → `Element::StatBlock` (shaded box)
- Fenced code blocks with `boxed` / `read-aloud` / `readaloud` → `Element::BoxedText`
- `<!-- 1-column -->` / `<!-- 2-column -->` HTML comments switch the page layout (the default is two-column)

When adding a new extension, the typical change touches three layers: detect it in `extensions.rs`, add an `Element` variant in `ast.rs`, and render it in `renderer/pdf.rs`.

### Statblock / boxed-text content

Inside a `statblock` or `boxed` fence, single newlines are soft (joined with a space, like a markdown paragraph); a blank line starts a new paragraph. Lines starting with `### ` / `#### ` are rendered as bold sub-headings on their own visual line. Bullet markers (`- `, `* `) become bullet items.

### Fonts

The renderer ships with vendored open fonts (OFL 1.1):

- `fonts/EBGaramond-{Regular,Italic,Bold,BoldItalic}.ttf` — body serif
- `fonts/JetBrainsMono-Regular.ttf` — inline `code` and code blocks

Embedded via `include_bytes!` in `src/renderer/pdf.rs`.

### Licenses

`src/licenses.rs` embeds the canonical OGL 1.0a and CC-BY-SA 4.0 texts via `include_str!` from `licenses/*.txt`. The body is parsed into setext-heading and paragraph fragments so that the source files' visual underlines (`====` / `----`) become real headings instead of literal characters in the output.

### Half-letter output

Page geometry (5.5" × 8.5", two-column, 0.4"/0.5" margins, 11pt column gap) is hardcoded in `src/renderer/pdf.rs` — the format is the project's identity, not a parameter.
