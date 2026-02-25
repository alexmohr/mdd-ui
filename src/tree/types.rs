/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

/// Type of top-level section
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionType {
    General,
    Variants,
    FunctionalGroups,
    EcuSharedData,
    Protocols,
}

/// Type of service list section
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceListType {
    DiagComms,
    Requests,
    PosResponses,
    NegResponses,
    FunctionalClasses,
}

/// Type of node for styling purposes
#[derive(Clone, Debug, PartialEq, Eq)]
// DOP, SDG are standard domain abbreviations
#[allow(clippy::upper_case_acronyms)]
pub enum NodeType {
    Container,
    SectionHeader,
    Service,
    ParentRefService, // Service inherited from parent reference
    ParentRefs,       // Parent References node
    Request,
    PosResponse,
    NegResponse,
    FunctionalClass, // Functional class node
    Job,             // Single ECU Job
    DOP,             // Data Object Property
    SDG,             // Special Data Group
    Default,
}

/// A single row in the flat tree view. Depth controls indentation, and
/// `expanded` / `has_children` drive the collapse/expand behaviour.
#[derive(Clone)]
pub struct TreeNode {
    pub depth: usize,
    pub text: String,
    pub expanded: bool,
    pub has_children: bool,
    pub detail_sections: Vec<DetailSectionData>,
    pub node_type: NodeType,
    /// If this is a `SectionHeader` at depth 0, specifies which top-level section it represents
    pub section_type: Option<SectionType>,
    /// If this is a `SectionHeader`, specifies which kind of service list it represents
    pub service_list_type: Option<ServiceListType>,
    /// If this is a parameter node, stores the parameter ID for lookup
    pub param_id: Option<u32>,
}

/// Type of detail section for logic purposes
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DetailSectionType {
    Header,          // Just a title/header section
    Overview,        // Overview table with key-value pairs
    Services,        // Services list table
    Requests,        // Request parameters
    PosResponses,    // Positive responses
    NegResponses,    // Negative responses
    ComParams,       // Communication parameters
    States,          // State information
    RelatedRefs,     // Related references
    FunctionalClass, // Functional class details
    Custom,          // Fallback for dynamic sections
}

/// Type of row for interaction purposes
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetailRowType {
    Normal,        // Regular data row
    Header,        // Table header row
    InheritedFrom, // "Inherited From" navigation row
    ChildElement,  // Child element summary row (clickable)
}

/// Type of child element in variant summary
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChildElementType {
    ComParamRefs,
    DiagComms,
    FunctionalClasses,
    NegResponses,
    PosResponses,
    Requests,
    SDGs,
    StateCharts,
}

impl ChildElementType {
    /// Get the display name for the child element type
    pub fn display_name(&self) -> &'static str {
        match self {
            ChildElementType::ComParamRefs => "ComParam Refs",
            ChildElementType::DiagComms => "Diag-Comms",
            ChildElementType::FunctionalClasses => "Functional Classes",
            ChildElementType::NegResponses => "Neg-Responses",
            ChildElementType::PosResponses => "Pos-Responses",
            ChildElementType::Requests => "Requests",
            ChildElementType::SDGs => "SDGs",
            ChildElementType::StateCharts => "State-Charts",
        }
    }

    /// Check if a node text starts with this child element type's display name
    pub fn matches_node_text(&self, text: &str) -> bool {
        text.starts_with(self.display_name())
    }
}

/// Metadata for special rows
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowMetadata {
    InheritedFrom { layer_name: String },
    ChildElement { element_type: ChildElementType },
    ParameterRow { param_id: u32 },
}

/// Type of cell content for interaction purposes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CellType {
    /// Regular text cell
    Text,
    /// Cell containing a DOP (Data Object Property) reference
    DopReference,
    /// Cell containing a numeric value
    NumericValue,
    /// Cell containing a parameter name
    ParameterName,
}

/// A row in a detail table
#[derive(Clone, Debug)]
pub struct DetailRow {
    pub cells: Vec<String>,
    pub cell_types: Vec<CellType>,
    pub indent: usize,
    pub row_type: DetailRowType,
    pub metadata: Option<RowMetadata>,
}

impl Default for DetailRow {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            cell_types: Vec::new(),
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        }
    }
}

/// Column constraint for table layout
#[derive(Clone, Debug)]
pub enum ColumnConstraint {
    /// Fixed width in characters
    Fixed(u16),
    /// Percentage of available width
    Percentage(u16),
}

/// Different types of content that can be displayed in a detail section
#[derive(Clone, Debug)]
pub enum DetailContent {
    /// Plain text lines (no table structure)
    PlainText(Vec<String>),
    /// A table with header, data rows, and column constraints
    Table {
        header: DetailRow,
        rows: Vec<DetailRow>,
        constraints: Vec<ColumnConstraint>,
        use_row_selection: bool,
    },
    /// Multiple subsections within a single tab, each with its own title and content
    Composite(Vec<DetailSectionData>),
}

impl DetailContent {
    /// Get a reference to the table rows, looking through `Composite` to find the first `Table`.
    pub fn table_rows(&self) -> Option<&Vec<DetailRow>> {
        match self {
            DetailContent::Table { rows, .. } => Some(rows),
            DetailContent::Composite(subs) => subs.iter().find_map(|s| s.content.table_rows()),
            DetailContent::PlainText(_) => None,
        }
    }

    /// Get the table constraints, looking through `Composite` to find the first `Table`.
    pub fn table_constraints(&self) -> Option<&Vec<ColumnConstraint>> {
        match self {
            DetailContent::Table { constraints, .. } => Some(constraints),
            DetailContent::Composite(subs) => {
                subs.iter().find_map(|s| s.content.table_constraints())
            }
            DetailContent::PlainText(_) => None,
        }
    }

    /// Get `use_row_selection`, looking through `Composite` to find the first `Table`.
    pub fn table_use_row_selection(&self) -> Option<bool> {
        match self {
            DetailContent::Table {
                use_row_selection, ..
            } => Some(*use_row_selection),
            DetailContent::Composite(subs) => subs
                .iter()
                .find_map(|s| s.content.table_use_row_selection()),
            DetailContent::PlainText(_) => None,
        }
    }

    /// Returns true if this content contains a table (directly or inside a Composite).
    pub fn has_table(&self) -> bool {
        match self {
            DetailContent::Table { .. } => true,
            DetailContent::Composite(subs) => subs.iter().any(|s| s.content.has_table()),
            DetailContent::PlainText(_) => false,
        }
    }

    /// Get the table header, looking through `Composite` to find the first `Table`.
    pub fn table_header(&self) -> Option<&DetailRow> {
        match self {
            DetailContent::Table { header, .. } => Some(header),
            DetailContent::Composite(subs) => subs.iter().find_map(|s| s.content.table_header()),
            DetailContent::PlainText(_) => None,
        }
    }
}

/// A detail section with a title and content
#[derive(Clone, Debug)]
pub struct DetailSectionData {
    pub title: String,
    pub content: DetailContent,
    /// If true, this section is rendered as a header above tabs, not as a tab itself
    pub render_as_header: bool,
    /// Type of section for logic purposes
    pub section_type: DetailSectionType,
}

impl DetailSectionData {
    /// Create a new `DetailSectionData` with Custom type by default
    pub fn new(title: String, content: DetailContent, render_as_header: bool) -> Self {
        Self {
            title,
            content,
            render_as_header,
            section_type: DetailSectionType::Custom,
        }
    }

    /// Create with a specific section type
    pub fn with_type(mut self, section_type: DetailSectionType) -> Self {
        self.section_type = section_type;
        self
    }
}

impl DetailRow {
    /// Create a normal data row
    pub fn normal(cells: Vec<String>, cell_types: Vec<CellType>, indent: usize) -> Self {
        Self {
            cells,
            cell_types,
            indent,
            row_type: DetailRowType::Normal,
            metadata: None,
        }
    }

    /// Create a table header row
    pub fn header(cells: Vec<String>, cell_types: Vec<CellType>) -> Self {
        Self {
            cells,
            cell_types,
            indent: 0,
            row_type: DetailRowType::Header,
            metadata: None,
        }
    }

    /// Create an "Inherited From" navigation row
    pub fn inherited_from(layer_name: String) -> Self {
        Self {
            cells: vec!["Inherited From".to_owned(), layer_name.clone()],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            row_type: DetailRowType::InheritedFrom,
            metadata: Some(RowMetadata::InheritedFrom { layer_name }),
        }
    }
}

/// Helper to create a plain text detail section
pub fn lines_to_single_section(title: &str, lines: Vec<String>) -> DetailSectionData {
    DetailSectionData::new(title.to_owned(), DetailContent::PlainText(lines), false)
}
