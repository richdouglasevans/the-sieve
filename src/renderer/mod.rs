pub mod html;
pub mod typst;

// Re-export HTML renderer as default (WeasyPrint for balanced columns)
pub use self::html::{compile_to_pdf, render_to_html, render_to_pdf};

// Typst renderer available under explicit module path
pub use self::typst::render_to_typst;
