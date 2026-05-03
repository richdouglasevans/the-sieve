# The Sieve

A CLI tool that converts TTRPG-flavored markdown into half-letter (5.5" × 8.5") PDFs sized for booklet printing.

## Features

- **Stat blocks** via fenced code blocks (`` ```statblock ``)
- **Boxed read-aloud text** (`` ```boxed ``)
- **Single-column override** (`<!-- 1-column -->` / `<!-- 2-column -->`) with mid-page mode switching
- **Manual page breaks** (`<!-- pagebreak -->`)
- **Two-column balanced layout** with H1 banners spanning both columns
- **License appendix** (`<!-- license: ogl-1.0a -->` / `<!-- license: cc-by-sa-4.0 -->`)
- Standard markdown: headings, lists, tables, images, emphasis, code blocks

See [`STYLE_GUIDE.md`](STYLE_GUIDE.md) for the full set of supported features and `sample.md` for a minimal example.

## Installation

```sh
cargo build --release
# binary lands at target/release/the-sieve
```

No runtime dependencies — fonts are embedded into the binary.

## Usage

```sh
the-sieve adventure.md                 # → adventure.pdf
the-sieve adventure.md -o booklet.pdf  # custom output path
the-sieve adventure.md --html-only     # emit intermediate HTML for debugging
```

## How it works

`markdown → AST → PDF`. The renderer is built on [krilla](https://crates.io/crates/krilla) (PDF output) and [parley](https://crates.io/crates/parley) (paragraph layout, line breaking, font fallback). Fonts shipped with the binary: EB Garamond and JetBrains Mono (both OFL 1.1).

## License

MIT — see [`LICENSE`](LICENSE).
