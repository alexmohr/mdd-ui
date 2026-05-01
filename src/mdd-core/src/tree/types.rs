/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use std::{fmt, sync::Arc};

use serde::Serialize;

/// Sentinel value for an unset bit position in the database.
pub(crate) const BIT_POSITION_UNSET: u32 = 255;

/// Strongly-typed prefixes embedded in tree node text to distinguish categories
/// that share the same parent node (e.g. services vs. jobs inside a Diag-Comm).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum NodeTextPrefix {
    /// Prefix for diagnostic service nodes: `"[Service] "`.
    Service,
    /// Prefix for job nodes: `"[Job] "`.
    Job,
}

impl NodeTextPrefix {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            NodeTextPrefix::Service => "[Service] ",
            NodeTextPrefix::Job => "[Job] ",
        }
    }
}

/// Type of top-level section in the tree hierarchy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ServiceListType {
    /// Communication parameter references.
    ComParamRefs,
    /// All diagnostic communication services.
    DiagComms,
    /// Functional class list.
    FunctionalClasses,
    /// Negative response service list.
    NegResponses,
    /// Positive response service list.
    PosResponses,
    /// Request-only service list.
    Requests,
    /// Special Data Group entries.
    SDGs,
    /// State chart definitions.
    StateCharts,
}

/// Type of node for styling and interaction purposes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize)]
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
    /// An individual normal (value-type) Data Object Property item.
    DopNormal,
    /// An individual DTC-DOP item.
    DopDtc,
    /// An individual Structure DOP item.
    DopStructure,
    /// An individual Static Field DOP item.
    DopStaticField,
    /// An individual Dynamic Length Field DOP item.
    DopDynamic,
    /// An individual End-of-PDU Field DOP item.
    DopEndOfPdu,
    /// An individual MUX DOP item.
    DopMux,
    /// An individual Env-Data DOP item.
    DopEnvData,
    /// An individual Env-Data-Desc DOP item.
    DopEnvDataDesc,
    /// Fallback node type with default styling.
    #[default]
    Default,
}

impl NodeType {
    /// Returns `true` for diagnostic-communication node types that represent a
    /// service entry (Service, `ParentRefService`, Request, `PosResponse`,
    /// `NegResponse`). Does **not** include `Job`.
    #[must_use]
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
    #[must_use]
    pub const fn is_diagcomm(self) -> bool {
        self.is_service() || matches!(self, NodeType::Job)
    }
}

/// Diff status for comparison mode.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
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

/// Extra data that only applies to certain [`NodeType`] variants.
///
/// Using an enum instead of a bag of `Option` fields shrinks the common-case
/// size of [`TreeNode`] and makes invariants explicit (e.g. only `Container`
/// nodes carry `parent_ref_names`).
#[derive(Clone, Debug, Default, Serialize)]
pub enum NodePayload {
    /// Container (variant / functional group / ECU shared data layer).
    Container {
        /// Canonical short name (identity comparisons, not display text).
        short_name: String,
        /// Short names of parent-ref containers from the database hierarchy.
        parent_ref_names: Vec<String>,
        /// Resolved tree indices of parent-ref containers. Populated by
        /// `TreeBuilder::finish()` from `parent_ref_names`.
        parent_ref_indices: Vec<usize>,
    },
    /// Top-level section header (depth 0).
    SectionHeader {
        /// Which top-level section this header represents.
        section_type: SectionType,
    },
    /// Service-list section header (e.g. Diag-Comms, Requests, …).
    ServiceListHeader {
        /// Which kind of service list this header represents.
        service_list_type: ServiceListType,
    },
    /// Diagcomm node (Service, `ParentRefService`, Request, Response, Job).
    DiagComm {
        /// Canonical short name used for sorting, matching and navigation.
        service_short_name: String,
    },
    /// Functional-class node.
    FunctionalClass {
        /// Canonical short name (identity comparisons, not display text).
        short_name: String,
    },
    /// Parameter node.
    Parameter {
        /// Database parameter ID.
        param_id: u32,
        /// Canonical short name (identity comparisons, not display text).
        short_name: String,
    },
    /// Fallback for nodes that carry no extra data.
    #[default]
    None,
}

/// A single row in the flat tree view. Depth controls indentation, and
/// `expanded` / `has_children` drive the collapse/expand behaviour.
#[derive(Clone, Debug, Serialize)]
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
    /// Enum-discriminated extra data for this node type.
    pub payload: NodePayload,
    /// Index into `all_nodes` of this node's direct parent, computed at build
    /// time. `None` for root (depth-0) nodes.  Enables O(1) parent lookups
    /// instead of O(n) backward scans.
    pub parent_idx: Option<usize>,
    /// Diff annotation for comparison mode. `None` in browse mode.
    pub diff_status: Option<DiffStatus>,
}

impl TreeNode {
    /// Returns the [`SectionType`] if this is a `SectionHeader` node.
    #[must_use]
    pub fn section_type(&self) -> Option<SectionType> {
        match &self.payload {
            NodePayload::SectionHeader { section_type } => Some(*section_type),
            _ => None,
        }
    }

    /// Returns the [`ServiceListType`] if this is a service-list header.
    #[must_use]
    pub fn service_list_type(&self) -> Option<ServiceListType> {
        match &self.payload {
            NodePayload::ServiceListHeader { service_list_type } => Some(*service_list_type),
            _ => None,
        }
    }

    /// Returns the parameter ID if this is a `Parameter` node.
    #[must_use]
    pub fn param_id(&self) -> Option<u32> {
        match &self.payload {
            NodePayload::Parameter { param_id, .. } => Some(*param_id),
            _ => None,
        }
    }

    /// Returns the resolved parent-ref container tree indices.
    #[must_use]
    pub fn parent_ref_indices(&self) -> &[usize] {
        match &self.payload {
            NodePayload::Container {
                parent_ref_indices, ..
            } => parent_ref_indices,
            _ => &[],
        }
    }

    /// Returns the canonical short name for nodes that carry one
    /// (`Container`, `FunctionalClass`, `Parameter`).
    #[must_use]
    pub fn short_name(&self) -> Option<&str> {
        match &self.payload {
            NodePayload::Container { short_name, .. }
            | NodePayload::FunctionalClass { short_name }
            | NodePayload::Parameter { short_name, .. } => Some(short_name),
            _ => None,
        }
    }

    /// Returns the canonical short name for diagcomm nodes.
    #[must_use]
    pub fn service_short_name(&self) -> Option<&str> {
        match &self.payload {
            NodePayload::DiagComm {
                service_short_name, ..
            } => Some(service_short_name),
            _ => None,
        }
    }
}

/// Type of detail section for logic and navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Default)]
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
    /// SOVD ECU lock management section.
    EcuLocks,
    /// Dynamic/fallback section type.
    #[default]
    Custom,
}

/// Type of row for interaction purposes.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
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
    #[must_use]
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

    /// Map this element type to the corresponding [`ServiceListType`].
    #[must_use]
    pub const fn to_service_list_type(&self) -> ServiceListType {
        match self {
            ChildElementType::ComParamRefs => ServiceListType::ComParamRefs,
            ChildElementType::DiagComms => ServiceListType::DiagComms,
            ChildElementType::FunctionalClasses => ServiceListType::FunctionalClasses,
            ChildElementType::NegResponses => ServiceListType::NegResponses,
            ChildElementType::PosResponses => ServiceListType::PosResponses,
            ChildElementType::Requests => ServiceListType::Requests,
            ChildElementType::SDGs => ServiceListType::SDGs,
            ChildElementType::StateCharts => ServiceListType::StateCharts,
        }
    }

    /// Check if a tree node's service-list type matches this element type.
    #[must_use]
    pub fn matches_node(&self, node: &TreeNode) -> bool {
        node.service_list_type() == Some(self.to_service_list_type())
    }
}

/// Metadata attached to special rows for navigation lookups.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum RowMetadata {
    /// Row represents an inherited element with the source layer name.
    InheritedFrom { layer_name: String },
    /// Row represents a child element of a specific type.
    ChildElement { element_type: ChildElementType },
    /// Row represents a parameter with the given database ID.
    ParameterRow { param_id: u32 },
}

/// Type of cell content for interaction purposes
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum CellJumpTargetType {
    /// Navigate to a parameter node by its ID
    Parameter { param_id: u32 },
    /// Navigate to a DOP node by its resolved tree index.
    /// `name` is the canonical short name used to resolve `index` via
    /// [`resolve_all_indices`](super::resolve_all_indices).
    Dop { index: usize, name: String },
    /// Navigate directly to a tree node by its stored index.
    /// Carries the canonical `short_name` for verification — if the node at
    /// `index` no longer matches (e.g. after sorting), the handler falls
    /// back to a hierarchy search by `short_name`.
    TreeNodeByIndex { index: usize, short_name: String },
    /// Navigate to a top-level Container node (Variant / Protocol / ECU Shared
    /// Data / Functional Group) by its canonical short name.  Unlike
    /// [`TreeNodeByIndex`], this variant resolves *only* against nodes whose
    /// payload carries a `short_name` (i.e. actual containers), never against
    /// plain-text child nodes that happen to share the same display text.
    Container { index: usize, short_name: String },
}

/// Per-cell jump target metadata: tells the navigation system where clicking
/// a jump cell should navigate to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CellJumpTarget {
    /// What kind of navigation this cell performs.
    pub target_type: CellJumpTargetType,
}

impl CellJumpTarget {
    /// Create a new jump target.
    #[must_use]
    pub fn new(target_type: CellJumpTargetType) -> Self {
        Self { target_type }
    }
}

/// A single cell in a detail table row.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DetailCell {
    /// Display text of the cell.
    pub text: String,
    /// Content type of the cell (controls styling).
    pub cell_type: CellType,
    /// Jump target for navigation. `None` means not navigable.
    pub jump_target: Option<CellJumpTarget>,
}

impl DetailCell {
    /// Create a cell with the given text and type, no jump target.
    pub fn new(text: impl Into<String>, cell_type: CellType) -> Self {
        Self {
            text: text.into(),
            cell_type,
            jump_target: None,
        }
    }

    /// Create a plain text cell.
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(text, CellType::Text)
    }

    /// Attach a jump target to this cell (builder pattern).
    #[must_use]
    pub fn with_jump(mut self, target: CellJumpTarget) -> Self {
        self.jump_target = Some(target);
        self
    }
}

/// A row in a detail table.
#[derive(Clone, Debug, Default, Serialize)]
pub struct DetailRow {
    /// Column cells for this row.
    pub cells: Vec<DetailCell>,
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
#[derive(Clone, Debug, Serialize)]
pub enum ColumnConstraint {
    /// Fixed width in characters
    Fixed(u16),
    /// Percentage of available width
    Percentage(u16),
}

/// Different types of content that can be displayed in a detail section
#[derive(Clone, Debug, Serialize)]
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

impl Default for DetailContent {
    fn default() -> Self {
        Self::PlainText(Vec::new())
    }
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
    #[must_use]
    pub fn table_rows(&self) -> Option<&[DetailRow]> {
        self.first_table().map(|(_, rows, _, _)| rows)
    }

    /// Get the table constraints, looking through `Composite` to find the first `Table`.
    #[must_use]
    pub fn table_constraints(&self) -> Option<&[ColumnConstraint]> {
        self.first_table().map(|(_, _, constraints, _)| constraints)
    }

    /// Get `use_row_selection`, looking through `Composite` to find the first `Table`.
    #[must_use]
    pub fn table_use_row_selection(&self) -> Option<bool> {
        self.first_table()
            .map(|(_, _, _, use_row_selection)| use_row_selection)
    }

    /// Get the table header, looking through `Composite` to find the first `Table`.
    #[must_use]
    pub fn table_header(&self) -> Option<&DetailRow> {
        self.first_table().map(|(header, _, _, _)| header)
    }
}

/// A detail section with a title and content.
#[derive(Clone, Debug, Serialize, Default)]
pub struct DetailSectionData {
    /// Display title of this section (shown as tab label).
    pub title: String,
    /// Body content (table, plain text, or composite).
    pub content: DetailContent,
    /// If true, this section is rendered as a header above tabs, not as a tab itself
    pub render_as_header: bool,
    /// Type of section for logic purposes
    pub section_type: DetailSectionType,
    /// Optional pre-computed byte/bit pattern rows for inline grid rendering.
    /// When `Some`, the frontend renders a `ByteGridView` below the main table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_pattern_rows: Option<Vec<DetailRow>>,
}

impl DetailSectionData {
    /// Create a new `DetailSectionData` with Custom type by default
    #[must_use]
    pub fn new(title: String, content: DetailContent, render_as_header: bool) -> Self {
        Self {
            title,
            content,
            render_as_header,
            section_type: DetailSectionType::Custom,
            byte_pattern_rows: None,
        }
    }

    /// Create with a specific section type
    #[must_use]
    pub fn with_type(mut self, section_type: DetailSectionType) -> Self {
        self.section_type = section_type;
        self
    }

    /// Attach pre-computed byte/bit pattern rows for inline grid rendering.
    #[must_use]
    pub fn with_byte_pattern_rows(mut self, rows: Vec<DetailRow>) -> Self {
        self.byte_pattern_rows = Some(rows);
        self
    }
}

impl DetailRow {
    /// Create a normal data row from pre-built cells.
    #[must_use]
    pub fn normal(cells: Vec<DetailCell>, indent: usize) -> Self {
        Self {
            cells,
            indent,
            row_type: DetailRowType::Normal,
            metadata: None,
            diff_status: None,
        }
    }

    /// Create a table header row from pre-built cells.
    #[must_use]
    pub fn header(cells: Vec<DetailCell>) -> Self {
        Self {
            cells,
            indent: 0,
            row_type: DetailRowType::Header,
            metadata: None,
            diff_status: None,
        }
    }

    /// Create an "Inherited From" navigation row.
    /// `container_index` is the tree index of the parent container, or
    /// `None` when the container has not been pushed yet.
    #[must_use]
    pub fn inherited_from(layer_name: String, container_index: Option<usize>) -> Self {
        let jump = CellJumpTarget::new(CellJumpTargetType::TreeNodeByIndex {
            index: container_index.unwrap_or(usize::MAX),
            short_name: layer_name.clone(),
        });
        Self {
            cells: vec![
                DetailCell::text("Inherited From"),
                DetailCell::new(layer_name.clone(), CellType::ParameterName).with_jump(jump),
            ],
            indent: 0,
            row_type: DetailRowType::InheritedFrom,
            metadata: Some(RowMetadata::InheritedFrom { layer_name }),
            diff_status: None,
        }
    }

    /// Convenience: get the text of cell at `idx`, or `""` if out of bounds.
    #[must_use]
    pub fn cell_text(&self, idx: usize) -> &str {
        self.cells.get(idx).map_or("", |c| c.text.as_str())
    }
}

/// Map a `ParamType` value from the database crate to a static display label.
///
/// Centralised here so that every call-site (tree building, snapshot
/// extraction, etc.) uses the same mapping.
#[must_use]
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
#[must_use]
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
    // ChildElementType::as_str / to_service_list_type
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
    fn to_service_list_type_mapping() {
        assert_eq!(
            ChildElementType::ComParamRefs.to_service_list_type(),
            ServiceListType::ComParamRefs
        );
        assert_eq!(
            ChildElementType::DiagComms.to_service_list_type(),
            ServiceListType::DiagComms
        );
        assert_eq!(
            ChildElementType::SDGs.to_service_list_type(),
            ServiceListType::SDGs
        );
        assert_eq!(
            ChildElementType::StateCharts.to_service_list_type(),
            ServiceListType::StateCharts
        );
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
