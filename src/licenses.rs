//! Embedded license texts referenced by the `<!-- license: ... -->` directive.

use crate::ast::LicenseKind;

const OGL_1_0A: &str = include_str!("../licenses/OGL-1.0a.txt");
const CC_BY_SA_4_0: &str = include_str!("../licenses/CC-BY-SA-4.0.txt");

pub fn body(kind: LicenseKind) -> &'static str {
    match kind {
        LicenseKind::Ogl1_0a => OGL_1_0A,
        LicenseKind::CcBySa4_0 => CC_BY_SA_4_0,
    }
}

pub enum LicenseFragment {
    /// `===` setext heading, e.g. major section
    Heading2(String),
    /// `---` setext heading, e.g. minor section
    Heading3(String),
    Paragraph(String),
}

/// Parse a license body into renderable fragments.
///
/// Recognizes setext-style headings (a non-empty line followed by a line of
/// `=` or `-` of length >= 3) and drops standalone separator lines. Non-heading
/// lines that are part of the same paragraph are joined with spaces.
pub fn fragments(kind: LicenseKind) -> Vec<LicenseFragment> {
    parse_body(body(kind))
}

fn is_separator(line: &str) -> bool {
    let t = line.trim();
    t.len() >= 3 && t.chars().all(|c| c == '=' || c == '-')
}

fn parse_body(text: &str) -> Vec<LicenseFragment> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            i += 1;
            continue;
        }
        // Setext heading: text line followed by underline.
        if i + 1 < lines.len() {
            let next = lines[i + 1].trim();
            if next.len() >= 3 {
                if next.chars().all(|c| c == '=') {
                    out.push(LicenseFragment::Heading2(line.to_string()));
                    i += 2;
                    continue;
                }
                if next.chars().all(|c| c == '-') {
                    out.push(LicenseFragment::Heading3(line.to_string()));
                    i += 2;
                    continue;
                }
            }
        }
        // Standalone separator: drop it.
        if is_separator(line) {
            i += 1;
            continue;
        }
        // Collect a paragraph: consecutive non-empty, non-separator lines.
        let mut para = String::new();
        while i < lines.len() {
            let l = lines[i].trim();
            if l.is_empty() || is_separator(l) {
                break;
            }
            // Stop if the next line is an underline (current line is a heading we missed).
            if i + 1 < lines.len() {
                let n = lines[i + 1].trim();
                if !para.is_empty() && n.len() >= 3 && n.chars().all(|c| c == '=' || c == '-') {
                    break;
                }
            }
            if !para.is_empty() {
                para.push(' ');
            }
            para.push_str(l);
            i += 1;
        }
        if !para.is_empty() {
            out.push(LicenseFragment::Paragraph(para));
        }
    }
    out
}

pub fn title(kind: LicenseKind) -> &'static str {
    match kind {
        LicenseKind::Ogl1_0a => "Open Game License v1.0a",
        LicenseKind::CcBySa4_0 => "Creative Commons Attribution-ShareAlike 4.0 International",
    }
}

pub fn short_name(kind: LicenseKind) -> &'static str {
    match kind {
        LicenseKind::Ogl1_0a => "OGL v1.0a",
        LicenseKind::CcBySa4_0 => "CC BY-SA 4.0",
    }
}

pub fn url(kind: LicenseKind) -> &'static str {
    match kind {
        LicenseKind::Ogl1_0a => "https://www.opengamingfoundation.org/ogl.html",
        LicenseKind::CcBySa4_0 => "https://creativecommons.org/licenses/by-sa/4.0/",
    }
}
