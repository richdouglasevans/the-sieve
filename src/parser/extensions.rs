use crate::ast::Image;
use std::path::PathBuf;

/// Detects and parses custom TTRPG extensions in markdown

/// Check if an HTML comment is a page break directive
pub fn is_page_break(html: &str) -> bool {
    let trimmed = html.trim();
    trimmed == "<!-- pagebreak -->" || trimmed == "<!--pagebreak-->"
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
