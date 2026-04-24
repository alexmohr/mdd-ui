/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use std::{fmt, sync::Arc};

/// Sentinel value for an unset bit position in the database.
pub(crate) const BIT_POSITION_UNSET: u32 = 255;

/// Strongly-typed prefixes embedded in tree node text to distinguish categories
/// that share the same parent node (e.g. services vs. jobs inside a Diag-Comm).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeTextPrefix {
    /// Prefix for diagnostic service nodes: `"[Service] "`.
    Service,
    /// Prefix for job nodes: `"[Job] "`.
    Job,
}

impl NodeTextPrefix {
    pub const fn as_str(self) -> &'static str {
        match self {
            NodeTextPrefix::Service => "[Service] ",
            NodeTextPrefix::Job => "[Job] ",
        }
    }
}

/// Type of top-level section in the tree hierarchy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionType {
    /// General information section (ECU name, metadata).
    General,
    /// Variant/layer definitions section.
    Variants,
    /// Functional group definitions section.
    FunctionalGroups,
    /// ECU shared data section.
    EcuSharedData,
    /// Communication protocols section.
    Protocols,
}

/// Type of service list section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceListType {
    /// All diagnostic communication services.
    DiagComms,
    /// Request-only service list.
    Requests,
    /// Positive response service list.
    PosResponses,
    /// Negative response service list.
    NegResponses,
    /// Functional class list.
    FunctionalClasses,
}

/// Type of node for styling and interaction purposes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum NodeType {
    /// Collapsible container without its own detail content.
    Container,
    /// Bold section header at a given depth.
    SectionHeader,
    /// A diagnostic service node.
    Service,
    /// A service inherited from a parent reference.
    ParentRefService,
    /// Parent references collection node.
    ParentRefs,
    /// A request definition node.
    Request,
    /// A positive response definition node.
    PosResponse,
    /// A negative response definition node.
    NegResponse,
    /// A functional class node.
    FunctionalClass,
    /// A single ECU job node.
    Job,
    /// A Data Object Property node.
    Dop,
    /// A Special Data Group node.
    Sdg,
    /// Fallback node type with default styling.
    #[default]
    Default,
}

impl NodeType {
    /// Returns `true` for diagnostic-communication node types that represent a
    /// service entry (Service, `ParentRefService`, Request, `PosResponse`,
    /// `NegResponse`). Does **not** include `Job`.
    pub const fn is_service(self) -> bool {
        matches!(
            self,
            NodeType::Service
                | NodeType::ParentRefService
                | NodeType::Request
                | NodeType::PosResponse
                | NodeType::NegResponse
        )
    }

    /// Returns `true` for nodes that live inside a Diag-Comms section:
    /// all [`is_service`](Self::is_service) types **plus** `Job`.
    pub const fn is_diagcomm(self) -> bool {
        self.is_service() || matches!(self, NodeType::Job)
    }
}

/// Diff status for comparison mode.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DiffStatus {
    /// Element exists only in the new file.
    Added,
    /// Element exists only in the old file.
    Removed,
    /// Element exists in both but differs.
    Modified,
    /// Element is identical in both files.
    Unchanged,
}

/// A single row in the flat tree view. Depth controls indentation, and
/// `expanded` / `has_children` drive the collapse/expand behaviour.
#[derive(Clone, Debug)]
pub struct TreeNode {
    /// Indentation level in the tree hierarchy (0 = root).
    pub depth: usize,
    /// Display text shown in the tree view.
    pub text: String,
    /// Whether this node is currently expanded to show its children.
    pub expanded: bool,
    /// Whether this node has child nodes that can be expanded.
    pub has_children: bool,
    /// Detail sections displayed when this node is selected.
    pub detail_sections: Arc<[DetailSectionData]>,
    /// Classification of this node for styling and interaction.
    pub node_type: NodeType,
    /// If this is a `SectionHeader` at depth 0, specifies which top-level section it represents
    pub section_type: Option<SectionType>,
    /// If this is a `SectionHeader`, specifies which kind of service list it represents
    pub service_list_type: Option<ServiceListType>,
    /// If this is a parameter node, stores the parameter ID for lookup
    pub param_id: Option<u32>,
    /// For `Container` nodes: short names of parent-ref containers from the
    /// database hierarchy. Used by the navigation system to walk the DB
    /// inheritance chain instead of scanning the tree.
    pub parent_ref_names: Vec<String>,
    /// Canonical short name for `Container` nodes, stored separately so that
    /// identity comparisons do not need to parse the display `text` (which
    /// may carry decorations like `" [base]"`).  `None` for non-container nodes.
    pub short_name: Option<String>,
    /// Index into `all_nodes` of this node's direct parent, computed at build
    /// time. `None` for root (depth-0) nodes.  Enables O(1) parent lookups
    /// instead of O(n) backward scans.
    pub parent_idx: Option<usize>,
    /// Diff annotation for comparison mode. `None` in browse mode.
    pub diff_status: Option<DiffStatus>,
}

/// Type of detail section for logic and navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DetailSectionType {
    /// Title-only header section rendered above tabs.
    Header,
    /// Key-value overview table.
    Overview,
    /// Services list table.
    Services,
    /// Request parameters table.
    Requests,
    /// Positive response parameters table.
    PosResponses,
    /// Negative response parameters table.
    NegResponses,
    /// Communication parameters section.
    ComParams,
    /// State information section.
    States,
    /// Related references section (parent refs, etc.).
    RelatedRefs,
    /// Functional class details section.
    FunctionalClass,
    /// Not-inherited `DiagComm` services list.
    NotInheritedDiagComms,
    /// Not-inherited Data Object Properties list.
    NotInheritedDops,
    /// Not-inherited Tables list.
    NotInheritedTables,
    /// Not-inherited `DiagVariables` list.
    NotInheritedVariables,
    /// Dynamic/fallback section type.
    Custom,
}

/// Type of row for interaction purposes.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum DetailRowType {
    /// Regular data row.
    #[default]
    Normal,
    /// Table header row.
    Header,
    /// "Inherited From" navigation row (clickable).
    InheritedFrom,
    /// Child element summary row (clickable navigation target).
    ChildElement,
}

/// Type of child element in a variant summary section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChildElementType {
    /// References to communication parameters.
    ComParamRefs,
    /// Diagnostic communication services.
    DiagComms,
    /// Functional class definitions.
    FunctionalClasses,
    /// Negative response definitions.
    NegResponses,
    /// Positive response definitions.
    PosResponses,
    /// Request definitions.
    Requests,
    /// Special Data Group entries.
    SDGs,
    /// State chart definitions.
    StateCharts,
}

impl fmt::Display for ChildElementType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ChildElementType {
    /// Return the display name as a static string without allocating.
    pub const fn as_str(&self) -> &'static str {
        match self {
            ChildElementType::ComParamRefs => "ComParam Refs",
            ChildElementType::DiagComms => "Diag-Comms",
            ChildElementType::FunctionalClasses => "Functional Classes",
            ChildElementType::NegResponses => "Neg-Responses",
            ChildElementType::PosResponses => "Pos-Responses",
            ChildElementType::Requests => "Requests",
            ChildElementType::SDGs => "SDGs",
            ChildElementType::StateCharts => "State Charts",
        }
    }

    /// Check if a node text starts with this child element type's display name
    pub fn matches_node_text(&self, text: &str) -> bool {
        text.starts_with(self.as_str())
    }
}

/// Metadata attached to special rows for navigation lookups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowMetadata {
    /// Row represents an inherited element with the source layer name.
    InheritedFrom { layer_name: String },
    /// Row represents a child element of a specific type.
    ChildElement { element_type: ChildElementType },
    /// Row represents a parameter with the given database ID.
    ParameterRow { param_id: u32 },
}

/// Type of cell content for interaction purposes
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum CellType {
    /// Regular text cell
    #[default]
    Text,
    /// Cell containing a DOP (Data Object Property) reference
    DopReference,
    /// Cell containing a numeric value
    NumericValue,
    /// Cell containing a parameter name
    ParameterName,
}

/// Classification of the navigation target for a jump cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellJumpTargetType {
    /// Navigate to a parameter node by its ID
    Parameter { param_id: u32 },
    /// Navigate to a DOP node by name
    Dop { name: String },
    /// Navigate to a tree node whose text matches the cell value
    TreeNodeByName,
    /// Navigate to a container (variant / layer) by name
    ContainerByName,
    /// Navigate to a Service or Job tree node whose short name matches the cell
    /// value. Service node texts have the format "ID - `ShortName`" so an exact
    /// `text == value` comparison does not work; this variant uses the
    /// dedicated service-name extraction logic instead.
    ServiceOrJobByName,
}

/// Per-cell jump target metadata: tells the navigation system where clicking
/// a jump cell should navigate to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellJumpTarget {
    /// What kind of navigation this cell performs.
    pub target_type: CellJumpTargetType,
}

impl CellJumpTarget {
    /// Create a new jump target.
    pub fn new(target_type: CellJumpTargetType) -> Self {
        Self { target_type }
    }
}

/// A row in a detail table.
#[derive(Clone, Debug, Default)]
pub struct DetailRow {
    /// Column values for this row.
    pub cells: Vec<String>,
    /// Content type of each cell (controls styling).
    pub cell_types: Vec<CellType>,
    /// Per-cell jump targets. Same length as `cells`; `None` means not navigable.
    pub cell_jump_targets: Vec<Option<CellJumpTarget>>,
    /// Indentation level for nested display.
    pub indent: usize,
    /// Semantic type of this row for interaction handling.
    pub row_type: DetailRowType,
    /// Optional metadata for navigation lookups.
    pub metadata: Option<RowMetadata>,
    /// Diff status for comparison mode.  `None` in browse mode.
    pub diff_status: Option<DiffStatus>,
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
    /// Locate the first `Table` variant, looking through `Composite` wrappers.
    /// Returns references to all four table fields so callers can project any subset.
    fn first_table(&self) -> Option<(&DetailRow, &[DetailRow], &[ColumnConstraint], bool)> {
        match self {
            DetailContent::Table {
                header,
                rows,
                constraints,
                use_row_selection,
            } => Some((header, rows, constraints, *use_row_selection)),
            DetailContent::Composite(subs) => subs.iter().find_map(|s| s.content.first_table()),
            DetailContent::PlainText(_) => None,
        }
    }

    /// Get a reference to the table rows, looking through `Composite` to find the first `Table`.
    pub fn table_rows(&self) -> Option<&[DetailRow]> {
        self.first_table().map(|(_, rows, _, _)| rows)
    }

    /// Get the table constraints, looking through `Composite` to find the first `Table`.
    pub fn table_constraints(&self) -> Option<&[ColumnConstraint]> {
        self.first_table().map(|(_, _, constraints, _)| constraints)
    }

    /// Get `use_row_selection`, looking through `Composite` to find the first `Table`.
    pub fn table_use_row_selection(&self) -> Option<bool> {
        self.first_table()
            .map(|(_, _, _, use_row_selection)| use_row_selection)
    }

    /// Get the table header, looking through `Composite` to find the first `Table`.
    pub fn table_header(&self) -> Option<&DetailRow> {
        self.first_table().map(|(header, _, _, _)| header)
    }
}

/// A detail section with a title and content.
#[derive(Clone, Debug)]
pub struct DetailSectionData {
    /// Display title of this section (shown as tab label).
    pub title: String,
    /// Body content (table, plain text, or composite).
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
        let jump_targets = vec![None; cells.len()];
        Self {
            cells,
            cell_types,
            cell_jump_targets: jump_targets,
            indent,
            row_type: DetailRowType::Normal,
            metadata: None,
            diff_status: None,
        }
    }

    /// Create a normal data row with per-cell jump targets
    pub fn with_jump_targets(
        cells: Vec<String>,
        cell_types: Vec<CellType>,
        cell_jump_targets: Vec<Option<CellJumpTarget>>,
        indent: usize,
    ) -> Self {
        Self {
            cells,
            cell_types,
            cell_jump_targets,
            indent,
            row_type: DetailRowType::Normal,
            metadata: None,
            diff_status: None,
        }
    }

    /// Create a table header row
    pub fn header(cells: Vec<String>, cell_types: Vec<CellType>) -> Self {
        let jump_targets = vec![None; cells.len()];
        Self {
            cells,
            cell_types,
            cell_jump_targets: jump_targets,
            indent: 0,
            row_type: DetailRowType::Header,
            metadata: None,
            diff_status: None,
        }
    }

    /// Create an "Inherited From" navigation row
    pub fn inherited_from(layer_name: String) -> Self {
        Self {
            cells: vec!["Inherited From".to_owned(), layer_name.clone()],
            cell_types: vec![CellType::Text, CellType::ParameterName],
            cell_jump_targets: vec![
                None,
                Some(CellJumpTarget::new(CellJumpTargetType::ContainerByName)),
            ],
            indent: 0,
            row_type: DetailRowType::InheritedFrom,
            metadata: Some(RowMetadata::InheritedFrom { layer_name }),
            diff_status: None,
        }
    }
}

/// Map a `ParamType` value from the database crate to a static display label.
///
/// Centralised here so that every call-site (tree building, snapshot
/// extraction, etc.) uses the same mapping.
pub fn param_type_label(pt: &cda_database::datatypes::ParamType) -> &'static str {
    use cda_database::datatypes::ParamType;
    match pt {
        ParamType::CodedConst => "CodedConst",
        ParamType::Dynamic => "Dynamic",
        ParamType::LengthKey => "LengthKey",
        ParamType::MatchingRequestParam => "MatchingRequestParam",
        ParamType::NrcConst => "NrcConst",
        ParamType::PhysConst => "PhysConst",
        ParamType::Reserved => "Reserved",
        ParamType::System => "System",
        ParamType::TableEntry => "TableEntry",
        ParamType::TableKey => "TableKey",
        ParamType::TableStruct => "TableStruct",
        ParamType::Value => "Value",
    }
}

/// Helper to create a plain text detail section
pub fn lines_to_single_section(title: &str, lines: Vec<String>) -> DetailSectionData {
    DetailSectionData::new(title.to_owned(), DetailContent::PlainText(lines), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // NodeType::is_service / is_diagcomm
    // ---------------------------------------------------------------

    #[test]
    fn is_service_includes_correct_variants() {
        assert!(NodeType::Service.is_service());
        assert!(NodeType::ParentRefService.is_service());
        assert!(NodeType::Request.is_service());
        assert!(NodeType::PosResponse.is_service());
        assert!(NodeType::NegResponse.is_service());
    }

    #[test]
    fn is_service_excludes_job_and_others() {
        assert!(!NodeType::Job.is_service());
        assert!(!NodeType::Container.is_service());
        assert!(!NodeType::SectionHeader.is_service());
        assert!(!NodeType::Dop.is_service());
        assert!(!NodeType::Default.is_service());
    }

    #[test]
    fn is_diagcomm_includes_service_plus_job() {
        assert!(NodeType::Service.is_diagcomm());
        assert!(NodeType::Job.is_diagcomm());
        assert!(NodeType::ParentRefService.is_diagcomm());
    }

    #[test]
    fn is_diagcomm_excludes_non_diagcomm() {
        assert!(!NodeType::Container.is_diagcomm());
        assert!(!NodeType::Dop.is_diagcomm());
        assert!(!NodeType::Default.is_diagcomm());
    }

    // ---------------------------------------------------------------
    // ChildElementType::as_str / matches_node_text
    // ---------------------------------------------------------------

    #[test]
    fn as_str_matches_display() {
        for elem in [
            ChildElementType::ComParamRefs,
            ChildElementType::DiagComms,
            ChildElementType::FunctionalClasses,
            ChildElementType::NegResponses,
            ChildElementType::PosResponses,
            ChildElementType::Requests,
            ChildElementType::SDGs,
            ChildElementType::StateCharts,
        ] {
            assert_eq!(elem.as_str(), elem.to_string());
        }
    }

    #[test]
    fn matches_node_text_positive() {
        assert!(ChildElementType::DiagComms.matches_node_text("Diag-Comms (5 services)"));
        assert!(ChildElementType::Requests.matches_node_text("Requests"));
    }

    #[test]
    fn matches_node_text_negative() {
        assert!(!ChildElementType::DiagComms.matches_node_text("Requests (3)"));
        assert!(!ChildElementType::Requests.matches_node_text("Pos-Responses"));
    }

    // ---------------------------------------------------------------
    // param_type_label
    // ---------------------------------------------------------------

    #[test]
    fn param_type_label_covers_all_variants() {
        use cda_database::datatypes::ParamType;
        let all = [
            (ParamType::CodedConst, "CodedConst"),
            (ParamType::Dynamic, "Dynamic"),
            (ParamType::LengthKey, "LengthKey"),
            (ParamType::MatchingRequestParam, "MatchingRequestParam"),
            (ParamType::NrcConst, "NrcConst"),
            (ParamType::PhysConst, "PhysConst"),
            (ParamType::Reserved, "Reserved"),
            (ParamType::System, "System"),
            (ParamType::TableEntry, "TableEntry"),
            (ParamType::TableKey, "TableKey"),
            (ParamType::TableStruct, "TableStruct"),
            (ParamType::Value, "Value"),
        ];
        for (pt, expected) in all {
            assert_eq!(param_type_label(&pt), expected);
        }
    }
}
