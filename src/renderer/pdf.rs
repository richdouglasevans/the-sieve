//! Native PDF renderer built on parley (paragraph layout) + krilla (PDF output).
//!
//! Phase 4: two-column layout with mid-page mode switching and balance.
//!
//! ## Tracks
//!
//! Document elements are grouped into "tracks" — runs of content with a single
//! column count (1 or 2) plus an optional trailing page break. `<!-- 1-column -->`
//! / `<!-- 2-column -->` directives split tracks; `<!-- pagebreak -->` terminates
//! a track with `page_break_after = true`. Each track's blocks are laid out at
//! the column width that applies to that track.
//!
//! ## Pagination
//!
//! Pages are filled top-to-bottom by stacking tracks. A track may span multiple
//! pages. Two-column tracks balance their content when it fits in the remaining
//! page space; on overflow, columns are filled greedily and the tail balances on
//! the final page.

use crate::ast::{
    Alignment as CellAlign, Document, Element, Image as AstImage, Inline, LicenseInfo,
    LicenseKind, Table,
};
use crate::licenses::{self, LicenseFragment};
use anyhow::{anyhow, Result};
use krilla::Document as KrillaDocument;
use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point, Rect, Size, Transform};
use krilla::image::Image;
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, Stroke};
use krilla::surface::Surface;
use krilla::text::{Font, GlyphId, KrillaGlyph};
use parley::{
    FontContext, Layout, LayoutContext,
    fontique::{Blob, Collection, CollectionOptions},
    layout::{Alignment as ParleyAlign, AlignmentOptions},
    style::{FontFamily, FontStack, FontStyle, FontWeight, LineHeight, StyleProperty},
};
use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::Arc;

const FONT_REGULAR: &[u8] = include_bytes!("../../fonts/EBGaramond-Regular.ttf");
const FONT_ITALIC: &[u8] = include_bytes!("../../fonts/EBGaramond-Italic.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../fonts/EBGaramond-Bold.ttf");
const FONT_BOLD_ITALIC: &[u8] = include_bytes!("../../fonts/EBGaramond-BoldItalic.ttf");
const FONT_MONO: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Regular.ttf");

const FAMILY: &str = "EB Garamond";
const MONO_FAMILY: &str = "JetBrains Mono";
const PAGE_W: f32 = 5.5 * 72.0;
const PAGE_H: f32 = 8.5 * 72.0;
const MARGIN_X: f32 = 0.4 * 72.0;
const MARGIN_Y: f32 = 0.5 * 72.0;
const BODY_SIZE: f32 = 9.0;
const LINE_HEIGHT: f32 = 1.4;

/// Horizontal gutter between columns in 2-column mode.
const COLUMN_GAP: f32 = 11.0;
/// One nesting level worth of horizontal indentation for lists / blockquotes.
const INDENT_STEP: f32 = 16.0;
/// Distance from the indent boundary to the marker glyph (marker sits to the
/// left of the indent line).
const LIST_MARKER_OFFSET: f32 = 10.0;
const TABLE_CELL_PAD_X: f32 = 6.0;
const TABLE_CELL_PAD_Y: f32 = 4.0;
const TABLE_BORDER_WIDTH: f32 = 0.5;
const TABLE_BORDER_COLOR: (u8, u8, u8) = (0x99, 0x99, 0x99);
const TABLE_HEADER_FILL: (u8, u8, u8) = (0xe8, 0xe8, 0xe8);

/// Render a Document AST to PDF bytes using the native pipeline.
pub fn render(doc: &Document, base_path: &Path) -> Result<Vec<u8>> {
    let mut font_cx = build_font_context();
    let mut layout_cx: LayoutContext<rgb::Color> = LayoutContext::new();

    // Pass 1: lay out elements into tracks.
    let tracks = build_tracks(doc, &mut font_cx, &mut layout_cx, base_path);

    // Pass 2: pack tracks onto pages.
    let mut document = KrillaDocument::new();
    let mut font_cache: HashMap<u64, Font> = HashMap::new();
    let mut headings: Vec<HeadingRecord> = Vec::new();

    let mut tracks: VecDeque<Track> = tracks.into();
    let mut emit_empty_page = tracks.is_empty();
    let mut page_index: usize = 0;

    while !tracks.is_empty() || emit_empty_page {
        emit_empty_page = false;

        let page_settings = PageSettings::from_wh(PAGE_W, PAGE_H)
            .ok_or_else(|| anyhow!("invalid page size"))?;
        let mut page = document.start_page_with(page_settings);
        let mut surface = page.surface();
        let mut y = MARGIN_Y;
        let content_bottom = PAGE_H - MARGIN_Y;

        while let Some(mut track) = tracks.pop_front() {
            let available = content_bottom - y;
            // Skip empty tracks (mode-changes with no content); honor page break.
            if track.blocks.is_empty() {
                if track.page_break_after {
                    break;
                }
                continue;
            }
            if available <= 0.0 {
                tracks.push_front(track);
                break;
            }
            let placed = match track.columns {
                1 => place_single_track(
                    &mut track,
                    &mut surface,
                    &mut y,
                    available,
                    &mut font_cache,
                    page_index,
                    &mut headings,
                ),
                _ => place_double_track(
                    &mut track,
                    &mut surface,
                    &mut y,
                    available,
                    &mut font_cache,
                    page_index,
                    &mut headings,
                ),
            };
            if !placed {
                tracks.push_front(track);
                break;
            }
            if track.page_break_after {
                break;
            }
        }

        draw_page_number(
            &mut surface,
            page_index + 1,
            &mut font_cx,
            &mut layout_cx,
            &mut font_cache,
        );

        surface.finish();
        page.finish();
        page_index += 1;
    }

    if !headings.is_empty() {
        document.set_outline(build_outline(&headings));
    }

    document.finish().map_err(|e| anyhow!("{e:?}"))
}

/// One heading captured during the pack pass — used to build a PDF outline
/// (the bookmarks tree shown in PDF viewer sidebars).
struct HeadingRecord {
    level: u8,
    text: String,
    page_index: usize,
    /// Top-left target point on the page (top of the heading line).
    x: f32,
    y: f32,
}

fn build_outline(records: &[HeadingRecord]) -> krilla::outline::Outline {
    use krilla::destination::XyzDestination;
    use krilla::geom::Point;
    use krilla::outline::{Outline, OutlineNode};

    let mut outline = Outline::new();
    // Stack of in-progress (level, node) pairs. We pop into the parent (or
    // outline root if no parent) when a heading at the same or shallower level
    // arrives, then push the new heading.
    let mut stack: Vec<(u8, OutlineNode)> = Vec::new();

    let close_to = |stack: &mut Vec<(u8, OutlineNode)>,
                    outline: &mut Outline,
                    until_level: u8| {
        while stack
            .last()
            .map(|(l, _)| *l >= until_level)
            .unwrap_or(false)
        {
            let (_, node) = stack.pop().unwrap();
            match stack.last_mut() {
                Some((_, parent)) => parent.push_child(node),
                None => outline.push_child(node),
            }
        }
    };

    for r in records {
        close_to(&mut stack, &mut outline, r.level);
        let dest = XyzDestination::new(r.page_index, Point::from_xy(r.x, r.y));
        stack.push((r.level, OutlineNode::new(r.text.clone(), dest)));
    }
    // Drain any remaining open nodes.
    close_to(&mut stack, &mut outline, 0);
    outline
}


/// A run of blocks with a single column count plus an optional trailing page
/// break. Tracks come from `build_tracks` and are consumed at pack time.
struct Track {
    columns: u8,
    blocks: Vec<LaidOut>,
    page_break_after: bool,
}

/// A block ready to draw, plus its measured geometry.
enum LaidOut {
    Block {
        layout: Layout<rgb::Color>,
        /// We must keep the source string alive for `draw_glyphs(&text)`.
        text: String,
        height: f32,
        space_after: f32,
        top_margin: f32,
        /// Left padding for content (added on top of the column origin).
        indent: f32,
        /// Optional list marker. Drawn to the left of the indent boundary,
        /// sharing the first-line baseline of the content layout.
        marker: Option<Marker>,
        /// Range of lines from `layout` to render (default 0..layout.len()).
        /// Allows a single laid-out paragraph to be drawn in pieces across
        /// columns/pages.
        line_start: usize,
        line_end: usize,
        /// True if the block can be split along line boundaries when it
        /// overflows. Headings, license titles, list-item placeholders are
        /// non-splittable.
        splittable: bool,
        /// If this is a heading (1-4), record so we can emit a PDF outline
        /// entry pointing at it.
        heading_level: Option<u8>,
    },
    Rule {
        height: f32,
        space_after: f32,
        top_margin: f32,
        /// Width to span (matches the column width at lay-out time).
        width: f32,
    },
    Table {
        cells: Vec<Vec<Cell>>,
        has_header: bool,
        column_widths: Vec<f32>,
        row_heights: Vec<f32>,
        height: f32,
        space_after: f32,
        top_margin: f32,
        indent: f32,
    },
    /// A boxed block — stat block or read-aloud "boxed text". Fixed geometry,
    /// fill + optional border, with one or more inner content layouts stacked
    /// inside.
    Boxed {
        inners: Vec<Inner>,
        width: f32,
        height: f32,
        space_after: f32,
        top_margin: f32,
        indent: f32,
        fill: Option<(u8, u8, u8)>,
        border: Option<((u8, u8, u8), f32)>,
        padding_x: f32,
        padding_y: f32,
    },
    Image {
        image: Image,
        width: f32,
        height: f32,
        space_after: f32,
        top_margin: f32,
        indent: f32,
    },
}

struct Marker {
    layout: Layout<rgb::Color>,
    text: String,
}

struct Cell {
    layout: Layout<rgb::Color>,
    text: String,
}

struct Inner {
    layout: Layout<rgb::Color>,
    text: String,
    /// X offset within the box's content area (after padding).
    inner_x: f32,
    /// Y offset within the box's content area (after padding).
    inner_y: f32,
    marker: Option<Marker>,
}

fn build_tracks(
    doc: &Document,
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<rgb::Color>,
    base_path: &Path,
) -> Vec<Track> {
    let mut tracks: Vec<Track> = Vec::new();
    let mut current_columns: u8 = 2;
    let mut current_blocks: Vec<LaidOut> = Vec::new();

    for el in &doc.elements {
        match el {
            Element::ColumnLayout(n) => {
                let n = (*n).clamp(1, 2);
                if n != current_columns {
                    if !current_blocks.is_empty() {
                        tracks.push(Track {
                            columns: current_columns,
                            blocks: std::mem::take(&mut current_blocks),
                            page_break_after: false,
                        });
                    }
                    current_columns = n;
                }
            }
            Element::PageBreak => {
                push_page_break_track(&mut tracks, &mut current_blocks, current_columns);
            }
            Element::License { kind, info } => {
                // Force the license appendix onto a fresh page, but skip the
                // page break if there's no content to flush and we're already
                // about to break (e.g. an immediately preceding pagebreak).
                push_page_break_track(&mut tracks, &mut current_blocks, current_columns);
                emit_license_tracks(*kind, info, font_cx, layout_cx, &mut tracks);
            }
            // H1 in 2-column mode spans both columns: split the surrounding
            // 2-col track so the H1 renders as a centered 1-col band.
            Element::Heading { level: 1, .. } if current_columns == 2 => {
                if !current_blocks.is_empty() {
                    tracks.push(Track {
                        columns: 2,
                        blocks: std::mem::take(&mut current_blocks),
                        page_break_after: false,
                    });
                }
                let mut span_blocks = Vec::new();
                let span_w = column_width_for(1);
                lay_out_element(el, 0.0, font_cx, layout_cx, span_w, base_path, &mut span_blocks);
                if let Some(LaidOut::Block { layout, .. }) = span_blocks.last_mut() {
                    layout.align(
                        Some(span_w),
                        ParleyAlign::Center,
                        AlignmentOptions::default(),
                    );
                }
                tracks.push(Track {
                    columns: 1,
                    blocks: span_blocks,
                    page_break_after: false,
                });
            }
            other => {
                let cw = column_width_for(current_columns);
                lay_out_element(other, 0.0, font_cx, layout_cx, cw, base_path, &mut current_blocks);
            }
        }
    }
    if !current_blocks.is_empty() {
        tracks.push(Track {
            columns: current_columns,
            blocks: current_blocks,
            page_break_after: false,
        });
    }
    tracks
}

/// Push a track that ends with a forced page break. If `current_blocks` is
/// empty AND the most recent track already ends with a page break, do nothing
/// — adjacent page-break directives must not produce blank pages between them.
fn push_page_break_track(
    tracks: &mut Vec<Track>,
    current_blocks: &mut Vec<LaidOut>,
    current_columns: u8,
) {
    if current_blocks.is_empty()
        && tracks
            .last()
            .map(|t| t.page_break_after)
            .unwrap_or(false)
    {
        return;
    }
    tracks.push(Track {
        columns: current_columns,
        blocks: std::mem::take(current_blocks),
        page_break_after: true,
    });
}

fn emit_license_tracks(
    kind: LicenseKind,
    info: &LicenseInfo,
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<rgb::Color>,
    tracks: &mut Vec<Track>,
) {
    let span_w = column_width_for(1);

    // 1) Header: title + optional attribution + optional changes (single column).
    let mut header_blocks: Vec<LaidOut> = Vec::new();

    header_blocks.push(make_centered_block(
        font_cx,
        layout_cx,
        licenses::title(kind),
        span_w,
        13.0,
        FontWeight::BOLD,
        FontStyle::Normal,
        BODY_SIZE * 0.5,
        BODY_SIZE * 0.5,
    ));

    if let Some(attr) = &info.attribution {
        let text = format!(
            "{} Licensed under {}. To view a copy of this license, visit {}.",
            attr,
            licenses::short_name(kind),
            licenses::url(kind),
        );
        header_blocks.push(make_centered_block(
            font_cx,
            layout_cx,
            &text,
            span_w,
            BODY_SIZE,
            FontWeight::BOLD,
            FontStyle::Normal,
            BODY_SIZE * 0.3,
            0.0,
        ));
    }
    if let Some(changes) = &info.changes {
        let text = format!("Changes from original: {}", changes);
        header_blocks.push(make_centered_block(
            font_cx,
            layout_cx,
            &text,
            span_w,
            BODY_SIZE,
            FontWeight::NORMAL,
            FontStyle::Italic,
            BODY_SIZE * 0.3,
            0.0,
        ));
    }

    tracks.push(Track {
        columns: 1,
        blocks: header_blocks,
        page_break_after: false,
    });

    // 2) Body: license fragments rendered at 6.5pt, two columns.
    let body_w = column_width_for(2);
    let mut body_blocks: Vec<LaidOut> = Vec::new();
    for frag in licenses::fragments(kind) {
        match frag {
            LicenseFragment::Heading2(text) => {
                let layout = build_layout(
                    font_cx,
                    layout_cx,
                    &text,
                    body_w,
                    8.0,
                    FontWeight::BOLD,
                    FontStyle::Normal,
                    &[],
                );
                body_blocks.push(make_block(layout, text, 0.0, 4.0, 4.0, false));
            }
            LicenseFragment::Heading3(text) => {
                let layout = build_layout(
                    font_cx,
                    layout_cx,
                    &text,
                    body_w,
                    7.0,
                    FontWeight::BOLD,
                    FontStyle::Normal,
                    &[],
                );
                body_blocks.push(make_block(layout, text, 0.0, 3.0, 3.0, false));
            }
            LicenseFragment::Paragraph(text) => {
                let layout = build_layout(
                    font_cx,
                    layout_cx,
                    &text,
                    body_w,
                    6.5,
                    FontWeight::NORMAL,
                    FontStyle::Normal,
                    &[],
                );
                body_blocks.push(make_block(layout, text, 0.0, 3.0, 0.0, true));
            }
        }
    }
    tracks.push(Track {
        columns: 2,
        blocks: body_blocks,
        page_break_after: false,
    });
}

/// Construct a `LaidOut::Block` from a fully-built parley `Layout`. Defaults
/// to no marker, `line_start = 0`, `line_end = layout.len()`.
fn make_block(
    layout: Layout<rgb::Color>,
    text: String,
    indent: f32,
    space_after: f32,
    top_margin: f32,
    splittable: bool,
) -> LaidOut {
    make_block_inner(layout, text, indent, space_after, top_margin, splittable, None)
}

fn make_heading_block(
    layout: Layout<rgb::Color>,
    text: String,
    indent: f32,
    space_after: f32,
    top_margin: f32,
    level: u8,
) -> LaidOut {
    make_block_inner(
        layout,
        text,
        indent,
        space_after,
        top_margin,
        false,
        Some(level),
    )
}

fn make_block_inner(
    layout: Layout<rgb::Color>,
    text: String,
    indent: f32,
    space_after: f32,
    top_margin: f32,
    splittable: bool,
    heading_level: Option<u8>,
) -> LaidOut {
    let line_end = layout.len();
    LaidOut::Block {
        height: layout.height(),
        layout,
        text,
        space_after,
        top_margin,
        indent,
        marker: None,
        line_start: 0,
        line_end,
        splittable,
        heading_level,
    }
}

fn make_centered_block(
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<rgb::Color>,
    text: &str,
    width: f32,
    size: f32,
    weight: FontWeight,
    style: FontStyle,
    space_after: f32,
    top_margin: f32,
) -> LaidOut {
    let mut layout = build_layout(font_cx, layout_cx, text, width, size, weight, style, &[]);
    layout.align(Some(width), ParleyAlign::Center, AlignmentOptions::default());
    make_block(layout, text.to_string(), 0.0, space_after, top_margin, false)
}

fn column_width_for(cols: u8) -> f32 {
    let total = PAGE_W - 2.0 * MARGIN_X;
    match cols {
        1 => total,
        _ => (total - COLUMN_GAP) / 2.0,
    }
}

/// Pack as many of the track's blocks as fit in the single column starting at
/// `*y_cursor`. Returns true if the track was fully placed.
fn place_single_track(
    track: &mut Track,
    surface: &mut Surface,
    y_cursor: &mut f32,
    available: f32,
    font_cache: &mut HashMap<u64, Font>,
    page_index: usize,
    headings: &mut Vec<HeadingRecord>,
) -> bool {
    let column_top = *y_cursor;
    let final_y = pack_into_column(
        &mut track.blocks,
        surface,
        MARGIN_X,
        column_top,
        available,
        font_cache,
        page_index,
        headings,
    );
    *y_cursor = if track.blocks.is_empty() {
        final_y
    } else {
        column_top + available
    };
    track.blocks.is_empty()
}

/// Greedily pack blocks (in order) into a single column starting at `column_top`
/// with `available` height. Splits paragraph blocks line-by-line when they
/// don't fit. Returns the final y after the last placed block. Drains placed
/// blocks from the front of `blocks`; what remains is for the next column/page.
fn pack_into_column(
    blocks: &mut Vec<LaidOut>,
    surface: &mut Surface,
    column_x: f32,
    column_top: f32,
    available: f32,
    font_cache: &mut HashMap<u64, Font>,
    page_index: usize,
    headings: &mut Vec<HeadingRecord>,
) -> f32 {
    let mut y = column_top;
    while !blocks.is_empty() {
        let at_top = (y - column_top).abs() < 0.001;
        let block = &blocks[0];
        let collapsed_top = if at_top { 0.0 } else { block_top_margin(block) };
        let draw_y = y + collapsed_top;
        let h = block_height(block);
        if draw_y + h > column_top + available {
            // Doesn't fit. Try to split.
            let block = blocks.remove(0);
            let remaining = (column_top + available - draw_y).max(0.0);
            match try_split_block(block, remaining) {
                Ok((top, bottom)) => {
                    let advance = block_height(&top) + block_space_after(&top);
                    draw_block(&top, surface, column_x, draw_y, font_cache, page_index, headings);
                    y = draw_y + advance;
                    if let Some(bottom) = bottom {
                        blocks.insert(0, bottom);
                        return y;
                    }
                }
                Err(original) => {
                    if at_top {
                        // Can't split and can't move — overflow it.
                        let advance = block_height(&original) + block_space_after(&original);
                        draw_block(&original, surface, column_x, draw_y, font_cache, page_index, headings);
                        y = draw_y + advance;
                    } else {
                        blocks.insert(0, original);
                        return y;
                    }
                }
            }
        } else {
            let block = blocks.remove(0);
            let advance = block_height(&block) + block_space_after(&block);
            draw_block(&block, surface, column_x, draw_y, font_cache, page_index, headings);
            y = draw_y + advance;
        }
    }
    y
}

/// Compute the visual height of lines `[line_start, line_end)` from a parley
/// layout — the gap from the slice's top to the slice's bottom.
fn slice_height(layout: &Layout<rgb::Color>, line_start: usize, line_end: usize) -> f32 {
    if line_end <= line_start {
        return 0.0;
    }
    let first = match layout.get(line_start) {
        Some(l) => l,
        None => return 0.0,
    };
    let last = match layout.get(line_end - 1) {
        Some(l) => l,
        None => return 0.0,
    };
    last.metrics().max_coord - first.metrics().min_coord
}

/// Try to split a block so that its top portion fits in `available` vertical
/// space. Returns `Ok((top, bottom?))` where `top` fits; if `bottom` is `Some`,
/// it carries the remaining lines. Returns `Err(original)` if the block isn't
/// splittable, or even one line wouldn't fit.
fn try_split_block(block: LaidOut, available: f32) -> Result<(LaidOut, Option<LaidOut>), LaidOut> {
    let LaidOut::Block {
        layout,
        text,
        height: _,
        space_after,
        top_margin,
        indent,
        marker,
        line_start,
        line_end,
        splittable,
        heading_level,
    } = block
    else {
        return Err(block);
    };
    if !splittable || line_end <= line_start + 1 {
        // Not splittable, or only one line — putting it back as-is.
        return Err(LaidOut::Block {
            height: slice_height(&layout, line_start, line_end),
            layout,
            text,
            space_after,
            top_margin,
            indent,
            marker,
            line_start,
            line_end,
            splittable,
            heading_level,
        });
    }

    // Find largest k in (line_start, line_end] such that slice [line_start, k)
    // fits in available height. Walk one line at a time.
    let mut k = line_start;
    for i in (line_start + 1)..=line_end {
        if slice_height(&layout, line_start, i) > available {
            break;
        }
        k = i;
    }
    if k <= line_start {
        // Even the first line doesn't fit.
        return Err(LaidOut::Block {
            height: slice_height(&layout, line_start, line_end),
            layout,
            text,
            space_after,
            top_margin,
            indent,
            marker,
            line_start,
            line_end,
            splittable,
            heading_level,
        });
    }
    if k >= line_end {
        // Whole thing fits.
        return Ok((
            LaidOut::Block {
                height: slice_height(&layout, line_start, line_end),
                layout,
                text,
                space_after,
                top_margin,
                indent,
                marker,
                line_start,
                line_end,
                splittable,
                heading_level,
            },
            None,
        ));
    }

    let top_h = slice_height(&layout, line_start, k);
    let bottom_h = slice_height(&layout, k, line_end);
    let top = LaidOut::Block {
        layout: layout.clone(),
        text: text.clone(),
        height: top_h,
        space_after: 0.0, // continuation comes immediately below; no gap
        top_margin,
        indent,
        marker,
        line_start,
        line_end: k,
        splittable,
        heading_level,
    };
    let bottom = LaidOut::Block {
        layout,
        text,
        height: bottom_h,
        space_after,
        top_margin: 0.0, // top margin only on the first piece
        indent,
        marker: None, // bullets only on the first piece
        line_start: k,
        line_end,
        splittable,
        // Outline entry only points at the first piece; continuation isn't a
        // separate heading.
        heading_level: None,
    };
    Ok((top, Some(bottom)))
}

/// Pack a 2-column track. If everything fits in the remaining vertical space
/// AND no single block is taller than `available`, balance the content between
/// columns (find a split that makes both columns roughly equal). Otherwise,
/// fall through to greedy fill (with paragraph splitting).
fn place_double_track(
    track: &mut Track,
    surface: &mut Surface,
    y_cursor: &mut f32,
    available: f32,
    font_cache: &mut HashMap<u64, Font>,
    page_index: usize,
    headings: &mut Vec<HeadingRecord>,
) -> bool {
    let n = track.blocks.len();
    if n == 0 {
        return true;
    }

    let col_w = column_width_for(2);
    let col0_x = MARGIN_X;
    let col1_x = MARGIN_X + col_w + COLUMN_GAP;
    let track_top = *y_cursor;

    let seq_heights: Vec<f32> = track
        .blocks
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let tm = if i == 0 { 0.0 } else { block_top_margin(b) };
            tm + block_height(b) + block_space_after(b)
        })
        .collect();
    let total: f32 = seq_heights.iter().sum();
    let max_block_h = track
        .blocks
        .iter()
        .map(block_height)
        .fold(0.0_f32, f32::max);

    if total <= 2.0 * available && max_block_h <= available {
        // Balance: find smallest k such that sum[0..k] >= total/2.
        let target = total / 2.0;
        let mut acc = 0.0_f32;
        let mut k = n;
        for (i, &h) in seq_heights.iter().enumerate() {
            if acc + h >= target {
                k = i + 1;
                break;
            }
            acc += h;
        }
        if k == 0 {
            k = 1;
        }
        let col0_y_after = place_blocks_into_column(
            &track.blocks[0..k],
            surface,
            col0_x,
            track_top,
            font_cache,
            page_index,
            headings,
        );
        let col1_y_after = place_blocks_into_column(
            &track.blocks[k..],
            surface,
            col1_x,
            track_top,
            font_cache,
            page_index,
            headings,
        );
        track.blocks.clear();
        let track_height = (col0_y_after - track_top).max(col1_y_after - track_top);
        *y_cursor = track_top + track_height;
        return true;
    }

    // Overflow / oversized-block case: greedy column 0, then column 1, with
    // paragraph splitting at line boundaries when a block doesn't fit whole.
    pack_into_column(
        &mut track.blocks,
        surface,
        col0_x,
        track_top,
        available,
        font_cache,
        page_index,
        headings,
    );
    if !track.blocks.is_empty() {
        pack_into_column(
            &mut track.blocks,
            surface,
            col1_x,
            track_top,
            available,
            font_cache,
            page_index,
            headings,
        );
    }
    *y_cursor = track_top + available;
    track.blocks.is_empty()
}

/// Draw a slice of blocks stacked vertically in a single column. Returns the y
/// after the last block's space_after.
fn place_blocks_into_column(
    blocks: &[LaidOut],
    surface: &mut Surface,
    column_x: f32,
    column_top: f32,
    font_cache: &mut HashMap<u64, Font>,
    page_index: usize,
    headings: &mut Vec<HeadingRecord>,
) -> f32 {
    let mut y = column_top;
    for (i, block) in blocks.iter().enumerate() {
        let at_top = i == 0;
        let collapsed_top = if at_top { 0.0 } else { block_top_margin(block) };
        let draw_y = y + collapsed_top;
        draw_block(block, surface, column_x, draw_y, font_cache, page_index, headings);
        y = draw_y + block_height(block) + block_space_after(block);
    }
    y
}

fn block_top_margin(b: &LaidOut) -> f32 {
    match b {
        LaidOut::Block { top_margin, .. }
        | LaidOut::Rule { top_margin, .. }
        | LaidOut::Table { top_margin, .. }
        | LaidOut::Boxed { top_margin, .. }
        | LaidOut::Image { top_margin, .. } => *top_margin,
    }
}

fn block_height(b: &LaidOut) -> f32 {
    match b {
        LaidOut::Block { height, .. }
        | LaidOut::Rule { height, .. }
        | LaidOut::Table { height, .. }
        | LaidOut::Boxed { height, .. }
        | LaidOut::Image { height, .. } => *height,
    }
}

fn block_space_after(b: &LaidOut) -> f32 {
    match b {
        LaidOut::Block { space_after, .. }
        | LaidOut::Rule { space_after, .. }
        | LaidOut::Table { space_after, .. }
        | LaidOut::Boxed { space_after, .. }
        | LaidOut::Image { space_after, .. } => *space_after,
    }
}

fn draw_block(
    block: &LaidOut,
    surface: &mut Surface,
    column_x: f32,
    draw_y: f32,
    font_cache: &mut HashMap<u64, Font>,
    page_index: usize,
    headings: &mut Vec<HeadingRecord>,
) {
    match block {
        LaidOut::Block {
            layout,
            text,
            indent,
            marker,
            line_start,
            line_end,
            heading_level,
            ..
        } => {
            let content_x = column_x + indent;
            draw_layout_slice(
                surface,
                layout,
                content_x,
                draw_y,
                *line_start,
                *line_end,
                text,
                font_cache,
            );
            if let Some(m) = marker {
                let marker_x = content_x - LIST_MARKER_OFFSET;
                draw_layout(surface, &m.layout, marker_x, draw_y, &m.text, font_cache);
            }
            if let Some(level) = *heading_level {
                headings.push(HeadingRecord {
                    level,
                    text: text.clone(),
                    page_index,
                    x: content_x,
                    y: draw_y,
                });
            }
        }
        LaidOut::Rule { width, .. } => {
            stroke_horizontal_line(
                surface,
                column_x,
                column_x + width,
                draw_y,
                1.0,
                TABLE_BORDER_COLOR,
            );
        }
        LaidOut::Table {
            cells,
            has_header,
            column_widths,
            row_heights,
            indent,
            ..
        } => {
            let table_left = column_x + indent;
            draw_table(
                surface,
                table_left,
                draw_y,
                cells,
                *has_header,
                column_widths,
                row_heights,
                font_cache,
            );
        }
        LaidOut::Boxed {
            inners,
            width,
            height,
            indent,
            fill,
            border,
            padding_x,
            padding_y,
            ..
        } => {
            draw_boxed(
                surface,
                column_x + indent,
                draw_y,
                *width,
                *height,
                inners,
                *fill,
                *border,
                *padding_x,
                *padding_y,
                font_cache,
            );
        }
        LaidOut::Image {
            image,
            width,
            height,
            indent,
            ..
        } => {
            if let Some(size) = Size::from_wh(*width, *height) {
                surface.push_transform(&Transform::from_translate(column_x + indent, draw_y));
                surface.draw_image(image.clone(), size);
                surface.pop();
            }
        }
    }
}

fn draw_boxed(
    surface: &mut Surface,
    box_x: f32,
    box_y: f32,
    width: f32,
    height: f32,
    inners: &[Inner],
    fill: Option<(u8, u8, u8)>,
    border: Option<((u8, u8, u8), f32)>,
    padding_x: f32,
    padding_y: f32,
    font_cache: &mut HashMap<u64, Font>,
) {
    if let Some((r, g, b)) = fill {
        if let Some(rect) = Rect::from_xywh(box_x, box_y, width, height) {
            let mut pb = PathBuilder::new();
            pb.push_rect(rect);
            if let Some(path) = pb.finish() {
                surface.set_stroke(None);
                surface.set_fill(Some(Fill {
                    paint: rgb::Color::new(r, g, b).into(),
                    opacity: NormalizedF32::ONE,
                    rule: Default::default(),
                }));
                surface.draw_path(&path);
            }
        }
    }
    if let Some(((r, g, b), w)) = border {
        if let Some(rect) = Rect::from_xywh(box_x, box_y, width, height) {
            let mut pb = PathBuilder::new();
            pb.push_rect(rect);
            if let Some(path) = pb.finish() {
                surface.set_fill(None);
                surface.set_stroke(Some(Stroke {
                    paint: rgb::Color::new(r, g, b).into(),
                    width: w,
                    ..Default::default()
                }));
                surface.draw_path(&path);
                surface.set_stroke(None);
            }
        }
    }
    let content_x = box_x + padding_x;
    let content_y = box_y + padding_y;
    for inner in inners {
        let x = content_x + inner.inner_x;
        let y = content_y + inner.inner_y;
        draw_layout(surface, &inner.layout, x, y, &inner.text, font_cache);
        if let Some(m) = &inner.marker {
            let marker_x = x - LIST_MARKER_OFFSET;
            draw_layout(surface, &m.layout, marker_x, y, &m.text, font_cache);
        }
    }
}

fn draw_table(
    surface: &mut Surface,
    table_left: f32,
    table_top: f32,
    cells: &[Vec<Cell>],
    has_header: bool,
    column_widths: &[f32],
    row_heights: &[f32],
    font_cache: &mut HashMap<u64, Font>,
) {
    let total_w: f32 = column_widths.iter().sum();
    let total_h: f32 = row_heights.iter().sum();

    if has_header && !row_heights.is_empty() {
        let rect = Rect::from_xywh(table_left, table_top, total_w, row_heights[0])
            .expect("non-zero header rect");
        let mut pb = PathBuilder::new();
        pb.push_rect(rect);
        if let Some(path) = pb.finish() {
            let (r, g, b) = TABLE_HEADER_FILL;
            surface.set_stroke(None);
            surface.set_fill(Some(Fill {
                paint: rgb::Color::new(r, g, b).into(),
                opacity: NormalizedF32::ONE,
                rule: Default::default(),
            }));
            surface.draw_path(&path);
        }
    }

    let mut row_y = table_top;
    for (row_idx, row) in cells.iter().enumerate() {
        let row_h = row_heights[row_idx];
        let mut col_x = table_left;
        for (col_idx, cell) in row.iter().enumerate() {
            let col_w = column_widths[col_idx];
            let text_y = row_y + TABLE_CELL_PAD_Y;
            draw_layout(
                surface,
                &cell.layout,
                col_x + TABLE_CELL_PAD_X,
                text_y,
                &cell.text,
                font_cache,
            );
            col_x += col_w;
        }
        row_y += row_h;
    }

    let (br, bg, bb) = TABLE_BORDER_COLOR;
    let stroke = Stroke {
        paint: rgb::Color::new(br, bg, bb).into(),
        width: TABLE_BORDER_WIDTH,
        ..Default::default()
    };
    surface.set_fill(None);
    surface.set_stroke(Some(stroke));

    {
        let mut pb = PathBuilder::new();
        pb.move_to(table_left, table_top);
        pb.line_to(table_left + total_w, table_top);
        pb.line_to(table_left + total_w, table_top + total_h);
        pb.line_to(table_left, table_top + total_h);
        pb.close();
        if let Some(path) = pb.finish() {
            surface.draw_path(&path);
        }
    }

    let mut yy = table_top;
    for &h in row_heights.iter().take(row_heights.len().saturating_sub(1)) {
        yy += h;
        let mut pb = PathBuilder::new();
        pb.move_to(table_left, yy);
        pb.line_to(table_left + total_w, yy);
        if let Some(path) = pb.finish() {
            surface.draw_path(&path);
        }
    }

    let mut xx = table_left;
    for &w in column_widths.iter().take(column_widths.len().saturating_sub(1)) {
        xx += w;
        let mut pb = PathBuilder::new();
        pb.move_to(xx, table_top);
        pb.line_to(xx, table_top + total_h);
        if let Some(path) = pb.finish() {
            surface.draw_path(&path);
        }
    }

    surface.set_stroke(None);
}

/// Draw a 1-based page number in the bottom margin. Odd pages get the number
/// in the bottom-right; even pages in the bottom-left — the standard book
/// convention so numbers face outward when bound.
fn draw_page_number(
    surface: &mut Surface,
    page_num: usize,
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<rgb::Color>,
    font_cache: &mut HashMap<u64, Font>,
) {
    let text = page_num.to_string();
    let size = 8.0;
    let max_advance = PAGE_W - 2.0 * MARGIN_X;
    let mut layout = build_layout(
        font_cx,
        layout_cx,
        &text,
        max_advance,
        size,
        FontWeight::NORMAL,
        FontStyle::Normal,
        &[],
    );
    let alignment = if page_num % 2 == 1 {
        ParleyAlign::Right
    } else {
        ParleyAlign::Start
    };
    layout.align(Some(max_advance), alignment, AlignmentOptions::default());

    // Vertically center the number in the bottom margin band.
    let layout_h = layout.height();
    let y = PAGE_H - MARGIN_Y + (MARGIN_Y - layout_h) / 2.0;
    draw_layout(surface, &layout, MARGIN_X, y, &text, font_cache);
}

fn stroke_horizontal_line(
    surface: &mut Surface,
    x_start: f32,
    x_end: f32,
    y: f32,
    width: f32,
    color: (u8, u8, u8),
) {
    let mut pb = PathBuilder::new();
    pb.move_to(x_start, y);
    pb.line_to(x_end, y);
    if let Some(path) = pb.finish() {
        let (r, g, b) = color;
        surface.set_fill(None);
        surface.set_stroke(Some(Stroke {
            paint: rgb::Color::new(r, g, b).into(),
            width,
            ..Default::default()
        }));
        surface.draw_path(&path);
        surface.set_stroke(None);
    }
}

fn lay_out_element(
    element: &Element,
    indent: f32,
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<rgb::Color>,
    content_width: f32,
    base_path: &Path,
    out: &mut Vec<LaidOut>,
) {
    let available = content_width - indent;
    match element {
        // Mode change and page break are handled by build_tracks; if they
        // appear nested (e.g. inside a blockquote), drop them silently.
        Element::PageBreak | Element::ColumnLayout(_) => {}
        Element::Heading { level, text } => {
            let size = heading_size(*level);
            let layout = build_layout(
                font_cx,
                layout_cx,
                text,
                available,
                size,
                FontWeight::BOLD,
                FontStyle::Normal,
                &[],
            );
            let (top, bottom) = heading_margins(*level, size);
            out.push(make_heading_block(
                layout,
                text.clone(),
                indent,
                bottom,
                top,
                *level,
            ));
        }
        Element::Paragraph(inlines) => {
            let (text, spans) = collect_inline_text(inlines);
            if text.is_empty() {
                return;
            }
            let layout = build_layout(
                font_cx,
                layout_cx,
                &text,
                available,
                BODY_SIZE,
                FontWeight::NORMAL,
                FontStyle::Normal,
                &spans,
            );
            out.push(make_block(layout, text, indent, BODY_SIZE * 0.6, 0.0, true));
        }
        Element::ThematicBreak => {
            out.push(LaidOut::Rule {
                height: 1.0,
                space_after: BODY_SIZE,
                top_margin: BODY_SIZE,
                width: available,
            });
        }
        Element::List { ordered, items } => {
            let item_indent = indent + INDENT_STEP;
            let item_available = content_width - item_indent;
            for (i, item) in items.iter().enumerate() {
                let marker_str = list_marker(*ordered, i, indent, item.task);
                let mut first_block = true;
                for child in &item.content {
                    let len_before = out.len();
                    lay_out_element(
                        child,
                        item_indent,
                        font_cx,
                        layout_cx,
                        content_width,
                        base_path,
                        out,
                    );
                    if first_block && out.len() > len_before {
                        attach_marker_to_first_block(
                            out,
                            len_before,
                            &marker_str,
                            font_cx,
                            layout_cx,
                        );
                        first_block = false;
                    }
                }
                if first_block {
                    let marker_layout = build_marker_layout(font_cx, layout_cx, &marker_str);
                    let placeholder = build_layout(
                        font_cx,
                        layout_cx,
                        " ",
                        item_available,
                        BODY_SIZE,
                        FontWeight::NORMAL,
                        FontStyle::Normal,
                        &[],
                    );
                    let mut placeholder_block = make_block(
                        placeholder,
                        " ".to_string(),
                        item_indent,
                        BODY_SIZE * 0.3,
                        0.0,
                        false,
                    );
                    if let LaidOut::Block { marker, .. } = &mut placeholder_block {
                        *marker = Some(Marker {
                            layout: marker_layout,
                            text: marker_str,
                        });
                    }
                    out.push(placeholder_block);
                }
            }
        }
        Element::BlockQuote(children) => {
            let bq_indent = indent + INDENT_STEP;
            for child in children {
                lay_out_element(
                    child,
                    bq_indent,
                    font_cx,
                    layout_cx,
                    content_width,
                    base_path,
                    out,
                );
            }
        }
        Element::Table(table) => {
            if let Some(laid_out) = lay_out_table(table, indent, font_cx, layout_cx, available) {
                out.push(laid_out);
            }
        }
        Element::StatBlock(content) => {
            out.push(lay_out_boxed(
                content,
                indent,
                available,
                Some((0xe8, 0xe8, 0xe8)), // gray fill
                None,                      // no border
                FontStyle::Normal,
                font_cx,
                layout_cx,
            ));
        }
        Element::BoxedText(content) => {
            out.push(lay_out_boxed(
                content,
                indent,
                available,
                Some((0xf4, 0xf4, 0xf0)),       // off-white fill
                Some(((0x99, 0x99, 0x99), 0.5)),// thin gray border
                FontStyle::Italic,
                font_cx,
                layout_cx,
            ));
        }
        Element::Image(img) => {
            if let Some(laid_out) = lay_out_image(img, indent, available, base_path) {
                out.push(laid_out);
            }
        }
        Element::CodeBlock { code, .. } => {
            out.push(lay_out_code_block(code, indent, available, font_cx, layout_cx));
        }
        // Element::License is handled at track-build time.
        _ => {}
    }
}

fn lay_out_code_block(
    code: &str,
    indent: f32,
    available: f32,
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<rgb::Color>,
) -> LaidOut {
    let padding_x: f32 = 8.0;
    let padding_y: f32 = 8.0;
    let inner_w = (available - 2.0 * padding_x).max(1.0);
    let size: f32 = 8.0;

    let span = InlineSpan {
        range: 0..code.len(),
        weight: FontWeight::NORMAL,
        style: FontStyle::Normal,
        monospace: true,
    };
    let layout = build_layout(
        font_cx,
        layout_cx,
        code,
        inner_w,
        size,
        FontWeight::NORMAL,
        FontStyle::Normal,
        std::slice::from_ref(&span),
    );
    let h = layout.height();

    let inners = vec![Inner {
        layout,
        text: code.to_string(),
        inner_x: 0.0,
        inner_y: 0.0,
        marker: None,
    }];
    let height = h + 2.0 * padding_y;

    LaidOut::Boxed {
        inners,
        width: available,
        height,
        space_after: BODY_SIZE * 0.5,
        top_margin: BODY_SIZE * 0.5,
        indent,
        fill: Some((0xf5, 0xf5, 0xf5)),
        border: None,
        padding_x,
        padding_y,
    }
}

fn lay_out_image(
    img: &AstImage,
    indent: f32,
    available: f32,
    base_path: &Path,
) -> Option<LaidOut> {
    let path = base_path.join(&img.path);
    let bytes = std::fs::read(&path).ok()?;
    let ext = img
        .path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());
    let krilla_img = match ext.as_deref() {
        Some("png") => Image::from_png(bytes.into(), false).ok()?,
        Some("jpg" | "jpeg") => Image::from_jpeg(bytes.into(), false).ok()?,
        Some("gif") => Image::from_gif(bytes.into(), false).ok()?,
        Some("webp") => Image::from_webp(bytes.into(), false).ok()?,
        _ => return None,
    };
    let (nat_w, nat_h) = krilla_img.size();
    if nat_w == 0 || nat_h == 0 {
        return None;
    }
    let target_w = parse_image_width(img.width.as_deref(), available).min(available);
    let scale = target_w / nat_w as f32;
    let target_h = nat_h as f32 * scale;
    Some(LaidOut::Image {
        image: krilla_img,
        width: target_w,
        height: target_h,
        space_after: BODY_SIZE * 0.5,
        top_margin: BODY_SIZE * 0.5,
        indent,
    })
}

fn parse_image_width(spec: Option<&str>, available: f32) -> f32 {
    let Some(spec) = spec else { return available };
    let s = spec.trim();
    if let Some(n) = s.strip_suffix('%') {
        let pct: f32 = n.trim().parse().unwrap_or(100.0);
        return available * pct / 100.0;
    }
    if let Some(n) = s.strip_suffix("pt") {
        return n.trim().parse().unwrap_or(available);
    }
    if let Some(n) = s.strip_suffix("px") {
        return n.trim().parse().unwrap_or(available);
    }
    s.parse().unwrap_or(available)
}

/// Lay out a stat-block or boxed-text container.
fn lay_out_boxed(
    content: &str,
    indent: f32,
    available: f32,
    fill: Option<(u8, u8, u8)>,
    border: Option<((u8, u8, u8), f32)>,
    base_style: FontStyle,
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<rgb::Color>,
) -> LaidOut {
    let padding_x: f32 = 8.0;
    let padding_y: f32 = 8.0;
    let inner_w = (available - 2.0 * padding_x).max(1.0);

    let items = parse_box_content(content);

    let mut inners: Vec<Inner> = Vec::new();
    let mut current_y: f32 = 0.0;
    let mut prev_kind: Option<&BoxItem> = None;

    for item in &items {
        if prev_kind.is_some() {
            current_y += match item {
                BoxItem::Bullet(_) if matches!(prev_kind, Some(BoxItem::Bullet(_))) => {
                    BODY_SIZE * 0.1
                }
                _ => BODY_SIZE * 0.4,
            };
        }
        match item {
            BoxItem::Paragraph(text) => {
                let (text, spans) = parse_inline_markdown(text);
                let layout = build_layout(
                    font_cx,
                    layout_cx,
                    &text,
                    inner_w,
                    BODY_SIZE,
                    FontWeight::NORMAL,
                    base_style,
                    &spans,
                );
                let h = layout.height();
                inners.push(Inner {
                    layout,
                    text,
                    inner_x: 0.0,
                    inner_y: current_y,
                    marker: None,
                });
                current_y += h;
            }
            BoxItem::Heading(text) => {
                let (text, _spans) = parse_inline_markdown(text);
                let layout = build_layout(
                    font_cx,
                    layout_cx,
                    &text,
                    inner_w,
                    BODY_SIZE,
                    FontWeight::BOLD,
                    FontStyle::Normal,
                    &[],
                );
                let h = layout.height();
                inners.push(Inner {
                    layout,
                    text,
                    inner_x: 0.0,
                    inner_y: current_y,
                    marker: None,
                });
                current_y += h;
            }
            BoxItem::Bullet(text) => {
                let bullet_indent: f32 = 12.0;
                let bullet_inner_w = (inner_w - bullet_indent).max(1.0);
                let (text, spans) = parse_inline_markdown(text);
                let layout = build_layout(
                    font_cx,
                    layout_cx,
                    &text,
                    bullet_inner_w,
                    BODY_SIZE,
                    FontWeight::NORMAL,
                    base_style,
                    &spans,
                );
                let h = layout.height();
                let marker_layout = build_marker_layout(font_cx, layout_cx, "•");
                inners.push(Inner {
                    layout,
                    text,
                    inner_x: bullet_indent,
                    inner_y: current_y,
                    marker: Some(Marker {
                        layout: marker_layout,
                        text: "•".to_string(),
                    }),
                });
                current_y += h;
            }
        }
        prev_kind = Some(item);
    }

    let content_height = current_y;
    let height = content_height + 2.0 * padding_y;

    LaidOut::Boxed {
        inners,
        width: available,
        height,
        space_after: BODY_SIZE * 0.5,
        top_margin: BODY_SIZE * 0.5,
        indent,
        fill,
        border,
        padding_x,
        padding_y,
    }
}

#[derive(Debug)]
enum BoxItem {
    Paragraph(String),
    Heading(String),
    Bullet(String),
}

/// Parse a stat-block / boxed-text content string into a sequence of items
/// (paragraph, sub-heading, bullet). Soft newlines within a paragraph join
/// with a space; blank lines split paragraphs. `### `/`#### ` prefix marks a
/// sub-heading; `- `/`* ` prefix marks a bullet (continuation lines append to
/// the previous bullet).
fn parse_box_content(content: &str) -> Vec<BoxItem> {
    let mut items: Vec<BoxItem> = Vec::new();
    for chunk in content.trim().split("\n\n") {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        let mut para_buf: Vec<String> = Vec::new();
        let mut bullets: Vec<String> = Vec::new();
        let mut in_bullets = false;

        for line in chunk.lines() {
            let line = line.trim();
            let bullet = line.strip_prefix("- ").or_else(|| line.strip_prefix("* "));
            let heading = line
                .strip_prefix("#### ")
                .or_else(|| line.strip_prefix("### "));

            if let Some(item) = bullet {
                if !para_buf.is_empty() {
                    items.push(BoxItem::Paragraph(para_buf.join(" ")));
                    para_buf.clear();
                }
                in_bullets = true;
                bullets.push(item.to_string());
            } else if let Some(h) = heading {
                if !para_buf.is_empty() {
                    items.push(BoxItem::Paragraph(para_buf.join(" ")));
                    para_buf.clear();
                }
                for b in bullets.drain(..) {
                    items.push(BoxItem::Bullet(b));
                }
                in_bullets = false;
                items.push(BoxItem::Heading(h.to_string()));
            } else if in_bullets {
                if let Some(last) = bullets.last_mut() {
                    last.push(' ');
                    last.push_str(line);
                }
            } else {
                para_buf.push(line.to_string());
            }
        }
        if !para_buf.is_empty() {
            items.push(BoxItem::Paragraph(para_buf.join(" ")));
        }
        for b in bullets {
            items.push(BoxItem::Bullet(b));
        }
    }
    items
}

/// Inline-markdown parser used for stat-block / boxed-text content (the AST
/// stores those as raw strings rather than `Vec<Inline>`). Handles `**bold**`,
/// `*italic*`, `\X` escape, and `->` typographic substitution. No nesting.
fn parse_inline_markdown(text: &str) -> (String, Vec<InlineSpan>) {
    let text = apply_typography(text);
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut spans: Vec<InlineSpan> = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            out.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_double_star(&chars, i + 2) {
                let start = out.len();
                let mut j = i + 2;
                while j < end {
                    if chars[j] == '\\' && j + 1 < end {
                        out.push(chars[j + 1]);
                        j += 2;
                    } else {
                        out.push(chars[j]);
                        j += 1;
                    }
                }
                let outer_end = out.len();
                spans.push(InlineSpan {
                    range: start..outer_end,
                    weight: FontWeight::BOLD,
                    style: FontStyle::Normal,
                    monospace: false,
                });
                i = end + 2;
                continue;
            }
        }
        if chars[i] == '*'
            && (i == 0 || chars[i - 1] != '*')
            && (i + 1 >= chars.len() || chars[i + 1] != '*')
        {
            if let Some(end) = find_single_star(&chars, i + 1) {
                let start = out.len();
                let mut j = i + 1;
                while j < end {
                    if chars[j] == '\\' && j + 1 < end {
                        out.push(chars[j + 1]);
                        j += 2;
                    } else {
                        out.push(chars[j]);
                        j += 1;
                    }
                }
                let outer_end = out.len();
                spans.push(InlineSpan {
                    range: start..outer_end,
                    weight: FontWeight::NORMAL,
                    style: FontStyle::Italic,
                    monospace: false,
                });
                i = end + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    (out, spans)
}

fn find_double_star(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i + 1 < chars.len() {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if chars[i] == '*' && chars[i + 1] == '*' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_single_star(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i < chars.len() {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if chars[i] == '*' {
            let prev = if i > 0 { chars[i - 1] } else { ' ' };
            let next = if i + 1 < chars.len() { chars[i + 1] } else { ' ' };
            if prev != '*' && next != '*' {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn apply_typography(text: &str) -> String {
    text.replace("->", "→")
}

fn lay_out_table(
    table: &Table,
    indent: f32,
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<rgb::Color>,
    available: f32,
) -> Option<LaidOut> {
    let n_cols = table
        .headers
        .len()
        .max(table.rows.iter().map(|r| r.len()).max().unwrap_or(0));
    if n_cols == 0 {
        return None;
    }
    let col_w = available / n_cols as f32;
    let cell_content_w = (col_w - 2.0 * TABLE_CELL_PAD_X).max(1.0);

    let mut all_rows: Vec<Vec<Cell>> = Vec::new();
    let has_header = !table.headers.is_empty();

    if has_header {
        let row = build_row_cells(
            &table.headers,
            n_cols,
            cell_content_w,
            &table.alignments,
            true,
            font_cx,
            layout_cx,
        );
        all_rows.push(row);
    }

    for row_strs in &table.rows {
        let row = build_row_cells(
            row_strs,
            n_cols,
            cell_content_w,
            &table.alignments,
            false,
            font_cx,
            layout_cx,
        );
        all_rows.push(row);
    }

    let row_heights: Vec<f32> = all_rows
        .iter()
        .map(|row| {
            let max_h = row
                .iter()
                .map(|c| c.layout.height())
                .fold(0.0_f32, f32::max);
            max_h + 2.0 * TABLE_CELL_PAD_Y
        })
        .collect();
    let height = row_heights.iter().sum::<f32>();

    let column_widths = vec![col_w; n_cols];
    Some(LaidOut::Table {
        cells: all_rows,
        has_header,
        column_widths,
        row_heights,
        height,
        space_after: BODY_SIZE * 0.5,
        top_margin: BODY_SIZE * 0.5,
        indent,
    })
}

fn build_row_cells(
    cells: &[String],
    n_cols: usize,
    cell_content_w: f32,
    alignments: &[CellAlign],
    is_header: bool,
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<rgb::Color>,
) -> Vec<Cell> {
    let mut row = Vec::with_capacity(n_cols);
    for col in 0..n_cols {
        let text = cells.get(col).cloned().unwrap_or_default();
        let align = alignments.get(col).copied().unwrap_or(CellAlign::None);
        let parley_align = match align {
            CellAlign::Center => ParleyAlign::Center,
            CellAlign::Right => ParleyAlign::Right,
            _ => ParleyAlign::Start,
        };
        let weight = if is_header {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        };
        let mut layout = build_layout(
            font_cx,
            layout_cx,
            &text,
            cell_content_w,
            BODY_SIZE,
            weight,
            FontStyle::Normal,
            &[],
        );
        layout.align(
            Some(cell_content_w),
            parley_align,
            AlignmentOptions::default(),
        );
        row.push(Cell { layout, text });
    }
    row
}

fn list_marker(ordered: bool, index: usize, indent: f32, task: Option<bool>) -> String {
    if let Some(checked) = task {
        if checked { "[x]".to_string() } else { "[ ]".to_string() }
    } else if ordered {
        format!("{}.", index + 1)
    } else if indent == 0.0 {
        "•".to_string()
    } else {
        "◦".to_string()
    }
}

fn build_marker_layout(
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<rgb::Color>,
    marker: &str,
) -> Layout<rgb::Color> {
    build_layout(
        font_cx,
        layout_cx,
        marker,
        LIST_MARKER_OFFSET * 4.0,
        BODY_SIZE,
        FontWeight::NORMAL,
        FontStyle::Normal,
        &[],
    )
}

fn attach_marker_to_first_block(
    out: &mut Vec<LaidOut>,
    start: usize,
    marker_str: &str,
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<rgb::Color>,
) {
    for block in &mut out[start..] {
        if let LaidOut::Block {
            marker, space_after, ..
        } = block
        {
            *marker = Some(Marker {
                layout: build_marker_layout(font_cx, layout_cx, marker_str),
                text: marker_str.to_string(),
            });
            *space_after = BODY_SIZE * 0.3;
            return;
        }
    }
}

fn heading_size(level: u8) -> f32 {
    match level {
        1 => 18.0,
        2 => 12.0,
        3 => 10.0,
        _ => 9.0,
    }
}

fn heading_margins(level: u8, size: f32) -> (f32, f32) {
    match level {
        1 => (0.5 * size, 0.8 * size),
        2 => (1.0 * size, 0.6 * size),
        3 => (0.8 * size, 0.5 * size),
        _ => (0.6 * size, 0.4 * size),
    }
}

fn build_font_context() -> FontContext {
    let mut collection = Collection::new(CollectionOptions {
        shared: false,
        system_fonts: false,
    });
    for bytes in [FONT_REGULAR, FONT_ITALIC, FONT_BOLD, FONT_BOLD_ITALIC, FONT_MONO] {
        collection.register_fonts(Blob::new(Arc::new(bytes.to_vec())), None);
    }
    FontContext {
        collection,
        source_cache: Default::default(),
    }
}

fn build_layout(
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<rgb::Color>,
    text: &str,
    max_advance: f32,
    size: f32,
    base_weight: FontWeight,
    base_style: FontStyle,
    spans: &[InlineSpan],
) -> Layout<rgb::Color> {
    let mut builder = layout_cx.ranged_builder(font_cx, text, 1.0, false);
    builder.push_default(StyleProperty::Brush(rgb::Color::new(0, 0, 0)));
    builder.push_default(StyleProperty::FontStack(FontStack::List(Cow::Borrowed(
        &[FontFamily::Named(Cow::Borrowed(FAMILY))],
    ))));
    builder.push_default(StyleProperty::FontSize(size));
    builder.push_default(StyleProperty::FontWeight(base_weight));
    builder.push_default(StyleProperty::FontStyle(base_style));
    builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
        LINE_HEIGHT,
    )));
    for span in spans {
        if span.weight != base_weight {
            builder.push(StyleProperty::FontWeight(span.weight), span.range.clone());
        }
        if span.style != base_style {
            builder.push(StyleProperty::FontStyle(span.style), span.range.clone());
        }
        if span.monospace {
            builder.push(
                StyleProperty::FontStack(FontStack::List(Cow::Borrowed(&[
                    FontFamily::Named(Cow::Borrowed(MONO_FAMILY)),
                ]))),
                span.range.clone(),
            );
        }
    }
    let mut layout = builder.build(text);
    layout.break_all_lines(Some(max_advance));
    layout
}

fn draw_layout(
    surface: &mut Surface,
    layout: &Layout<rgb::Color>,
    origin_x: f32,
    origin_y: f32,
    text: &str,
    font_cache: &mut HashMap<u64, Font>,
) {
    let len = layout.len();
    draw_layout_slice(
        surface,
        layout,
        origin_x,
        origin_y,
        0,
        len,
        text,
        font_cache,
    );
}

fn draw_layout_slice(
    surface: &mut Surface,
    layout: &Layout<rgb::Color>,
    origin_x: f32,
    origin_y: f32,
    line_start: usize,
    line_end: usize,
    text: &str,
    font_cache: &mut HashMap<u64, Font>,
) {
    if line_end <= line_start {
        return;
    }
    // Shift so the first line of the slice appears at `origin_y`.
    let y_shift = match layout.get(line_start) {
        Some(l) => origin_y - l.metrics().min_coord,
        None => return,
    };

    surface.set_fill(Some(Fill {
        paint: rgb::Color::new(0, 0, 0).into(),
        opacity: NormalizedF32::ONE,
        rule: Default::default(),
    }));

    for i in line_start..line_end {
        let Some(line) = layout.get(i) else { break };
        let baseline = y_shift + line.metrics().baseline;
        let mut x = origin_x + line.metrics().offset;
        for run in line.runs() {
            let parley_font = run.font().clone();
            let (font_data, font_id) = parley_font.data.into_raw_parts();
            let font_size = run.font_size();
            let krilla_font = font_cache
                .entry(font_id)
                .or_insert_with(|| {
                    Font::new(font_data.into(), parley_font.index as u32).unwrap()
                })
                .clone();

            let run_start_x = x;
            let mut glyphs: Vec<KrillaGlyph> = Vec::new();

            for cluster in run.visual_clusters() {
                if cluster.is_ligature_continuation() {
                    if let Some(g) = glyphs.last_mut() {
                        g.text_range.end = cluster.text_range().end;
                    }
                    continue;
                }
                for glyph in cluster.glyphs() {
                    glyphs.push(KrillaGlyph::new(
                        GlyphId::new(glyph.id as u32),
                        glyph.advance / font_size,
                        glyph.x / font_size,
                        glyph.y / font_size,
                        0.0,
                        cluster.text_range(),
                        None,
                    ));
                    x += glyph.advance;
                }
            }

            if !glyphs.is_empty() {
                surface.draw_glyphs(
                    Point::from_xy(run_start_x, baseline),
                    &glyphs,
                    krilla_font,
                    text,
                    font_size,
                    false,
                );
            }
        }
    }
}

#[derive(Debug, Clone)]
struct InlineSpan {
    range: std::ops::Range<usize>,
    weight: FontWeight,
    style: FontStyle,
    monospace: bool,
}

fn collect_inline_text(inlines: &[Inline]) -> (String, Vec<InlineSpan>) {
    let mut text = String::new();
    let mut spans = Vec::new();
    for inline in inlines {
        walk_inline(
            inline,
            &mut text,
            &mut spans,
            FontWeight::NORMAL,
            FontStyle::Normal,
        );
    }
    (text, spans)
}

fn walk_inline(
    inline: &Inline,
    text: &mut String,
    spans: &mut Vec<InlineSpan>,
    weight: FontWeight,
    style: FontStyle,
) {
    match inline {
        Inline::Text(t) => {
            let start = text.len();
            text.push_str(t);
            let end = text.len();
            if weight != FontWeight::NORMAL || style != FontStyle::Normal {
                spans.push(InlineSpan {
                    range: start..end,
                    weight,
                    style,
                    monospace: false,
                });
            }
        }
        Inline::Code(t) => {
            let start = text.len();
            text.push_str(t);
            let end = text.len();
            spans.push(InlineSpan {
                range: start..end,
                weight,
                style,
                monospace: true,
            });
        }
        Inline::Emphasis(children) => {
            for c in children {
                walk_inline(c, text, spans, weight, FontStyle::Italic);
            }
        }
        Inline::Strong(children) => {
            for c in children {
                walk_inline(c, text, spans, FontWeight::BOLD, style);
            }
        }
        Inline::Link {
            text: link_text, ..
        } => {
            for c in link_text {
                walk_inline(c, text, spans, weight, style);
            }
        }
        Inline::Image(_) => {}
        Inline::SoftBreak => text.push(' '),
        Inline::HardBreak => text.push('\n'),
    }
}
