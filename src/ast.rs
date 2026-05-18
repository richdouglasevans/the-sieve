use std::path::PathBuf;

/// Intermediate document representation for TTRPG content
#[derive(Debug, Clone)]
pub struct Document {
    pub elements: Vec<Element>,
}

impl Document {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    pub fn push(&mut self, element: Element) {
        self.elements.push(element);
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum Element {
    Heading { level: u8, text: String },
    Paragraph(Vec<Inline>),
    CodeBlock { language: Option<String>, code: String },
    BlockQuote(Vec<Element>),
    List { ordered: bool, items: Vec<ListItem> },
    ThematicBreak,
    PageBreak,
    ColumnLayout(u8), // Number of columns (1 = single, 2 = two-column)
    StatBlock(String), // Shaded box for stat blocks
    BoxedText(String),
    Image(Image),
    Table(Table),
    License { kind: LicenseKind, info: LicenseInfo },
    Raw(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseKind {
    Ogl1_0a,
    CcBySa4_0,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LicenseInfo {
    pub attribution: Option<String>,
    pub changes: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Code(String),
    Link { text: Vec<Inline>, url: String },
    Image(Image),
    SoftBreak,
    HardBreak,
}

#[derive(Debug, Clone)]
pub struct ListItem {
    pub content: Vec<Element>,
    /// None = ordinary list item; Some(false) = unchecked task; Some(true) = checked task.
    pub task: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Image {
    pub alt: String,
    pub path: PathBuf,
    pub width: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Table {
    pub headers: Vec<String>,
    pub alignments: Vec<Alignment>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Copy)]
pub enum Alignment {
    Left,
    Center,
    Right,
    None,
}

