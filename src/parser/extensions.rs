use crate::ast::{Image, LicenseInfo, LicenseKind};
use std::path::PathBuf;

/// Detects and parses custom TTRPG extensions in markdown

/// Check if an HTML comment is a page break directive
pub fn is_page_break(html: &str) -> bool {
    let trimmed = html.trim();
    trimmed == "<!-- pagebreak -->" || trimmed == "<!--pagebreak-->"
}

/// Check if an HTML comment is a license directive (e.g. `<!-- license: ogl-1.0a -->`
/// or `<!-- license: cc-by-sa-4.0 attribution="X" changes="Y" -->`).
pub fn parse_license(html: &str) -> Option<(LicenseKind, LicenseInfo)> {
    let inner = html
        .trim()
        .strip_prefix("<!--")?
        .strip_suffix("-->")?
        .trim();
    let value = inner.strip_prefix("license:")?.trim();
    let (kind_str, rest) = value
        .split_once(|c: char| c.is_ascii_whitespace())
        .unwrap_or((value, ""));
    let kind = match kind_str.to_lowercase().as_str() {
        "ogl-1.0a" | "ogl1.0a" | "ogl" => LicenseKind::Ogl1_0a,
        "cc-by-sa-4.0" | "ccbysa4.0" | "cc-by-sa" => LicenseKind::CcBySa4_0,
        _ => return None,
    };
    let info = LicenseInfo {
        attribution: extract_quoted(rest, "attribution"),
        changes: extract_quoted(rest, "changes"),
    };
    Some((kind, info))
}

/// Extract a `key="value"` pair from a directive tail; quotes do not nest or escape.
fn extract_quoted(s: &str, key: &str) -> Option<String> {
    let needle = format!("{}=\"", key);
    let start = s.find(&needle)? + needle.len();
    let rest = &s[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Check if an HTML comment is a column layout directive
/// Returns Some(columns) if it's a column directive, None otherwise
pub fn parse_column_layout(html: &str) -> Option<u8> {
    let trimmed = html.trim();

    if trimmed == "<!-- 1-column -->" {
        Some(1)
    } else if trimmed == "<!-- 2-column -->" {
        Some(2)
    } else {
        None
    }
}

/// Parse boxed text (read-aloud text)
pub fn parse_boxed_text(content: &str) -> String {
    content.trim().to_string()
}

/// Parse columns from content separated by ---
/// Parse image attributes from markdown image syntax
/// Supports: ![alt](path){width=50%}
pub fn parse_image_attributes(alt: &str, url: &str, title: Option<&str>) -> Image {
    let mut width = None;
    let mut path = url.to_string();

    // Check for width attribute in the URL (after closing paren, in braces)
    // This is handled during markdown parsing, but we can also check title
    if let Some(t) = title {
        if t.starts_with("width=") {
            width = Some(t.trim_start_matches("width=").to_string());
        }
    }

    // Check for inline attributes like {width=50%}
    if let Some(brace_start) = url.find('{') {
        path = url[..brace_start].to_string();
        let attrs = &url[brace_start + 1..url.len().saturating_sub(1)];
        for attr in attrs.split(',') {
            let attr = attr.trim();
            if let Some(w) = attr.strip_prefix("width=") {
                width = Some(w.to_string());
            }
        }
    }

    Image {
        alt: alt.to_string(),
        path: PathBuf::from(path),
        width,
    }
}

/// Detect the type of fenced code block
#[derive(Debug, Clone, PartialEq)]
pub enum CodeBlockType {
    StatBlock,
    Boxed,
    Regular(Option<String>),
}

pub fn detect_code_block_type(info_string: &str) -> CodeBlockType {
    match info_string.trim().to_lowercase().as_str() {
        "statblock" | "stat-block" | "monster" => CodeBlockType::StatBlock,
        "boxed" | "read-aloud" | "readaloud" => CodeBlockType::Boxed,
        "" => CodeBlockType::Regular(None),
        other => CodeBlockType::Regular(Some(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_break_detection() {
        assert!(is_page_break("<!-- pagebreak -->"));
        assert!(is_page_break("<!--pagebreak-->"));
        assert!(is_page_break("  <!-- pagebreak -->  "));
        assert!(!is_page_break("<!-- other -->"));
    }

    #[test]
    fn test_column_layout_detection() {
        assert_eq!(parse_column_layout("<!-- 1-column -->"), Some(1));
        assert_eq!(parse_column_layout("<!-- 2-column -->"), Some(2));
        assert_eq!(parse_column_layout("<!-- other -->"), None);
    }

    #[test]
    fn test_license_directive_detection() {
        let bare = LicenseInfo::default();
        assert_eq!(
            parse_license("<!-- license: ogl-1.0a -->"),
            Some((LicenseKind::Ogl1_0a, bare.clone()))
        );
        assert_eq!(
            parse_license("<!--license:ogl-->"),
            Some((LicenseKind::Ogl1_0a, bare.clone()))
        );
        assert_eq!(
            parse_license("<!-- license: cc-by-sa-4.0 -->"),
            Some((LicenseKind::CcBySa4_0, bare.clone()))
        );
        assert_eq!(parse_license("<!-- pagebreak -->"), None);
        assert_eq!(parse_license("<!-- license: unknown -->"), None);
    }

    #[test]
    fn test_license_attribution_parsing() {
        let parsed =
            parse_license(r#"<!-- license: cc-by-sa-4.0 attribution="X by Y" changes="reformatted" -->"#)
                .unwrap();
        assert_eq!(parsed.0, LicenseKind::CcBySa4_0);
        assert_eq!(parsed.1.attribution.as_deref(), Some("X by Y"));
        assert_eq!(parsed.1.changes.as_deref(), Some("reformatted"));
    }

    #[test]
    fn test_license_partial_attribution() {
        let parsed =
            parse_license(r#"<!-- license: cc-by-sa-4.0 attribution="Just attribution" -->"#)
                .unwrap();
        assert_eq!(parsed.1.attribution.as_deref(), Some("Just attribution"));
        assert_eq!(parsed.1.changes, None);
    }

    #[test]
    fn test_code_block_type_detection() {
        assert_eq!(detect_code_block_type("statblock"), CodeBlockType::StatBlock);
        assert_eq!(detect_code_block_type("stat-block"), CodeBlockType::StatBlock);
        assert_eq!(detect_code_block_type("monster"), CodeBlockType::StatBlock);
        assert_eq!(detect_code_block_type("boxed"), CodeBlockType::Boxed);
        assert_eq!(detect_code_block_type("read-aloud"), CodeBlockType::Boxed);
        assert_eq!(detect_code_block_type(""), CodeBlockType::Regular(None));
        assert_eq!(detect_code_block_type("rust"), CodeBlockType::Regular(Some("rust".to_string())));
    }
}
