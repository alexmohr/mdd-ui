/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::types::{DetailSectionData, NodeType, SectionType, ServiceListType, TreeNode};

/// Accumulates `TreeNode`s while walking the database model.
///
/// Methods are spread across submodules (`services`, `layers`) via
/// `impl TreeBuilder` blocks so each concern lives in its own file.
pub struct TreeBuilder {
    nodes: Vec<TreeNode>,
}

impl TreeBuilder {
    pub(crate) fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Push a node that carries structured detail sections (preferred).
    pub(crate) fn push_details_structured(
        &mut self,
        depth: usize,
        text: String,
        expanded: bool,
        has_children: bool,
        sections: Vec<DetailSectionData>,
        node_type: NodeType,
    ) {
        self.nodes.push(TreeNode {
            depth,
            text,
            expanded,
            has_children,
            detail_sections: sections,
            node_type,
            section_type: None,
            service_list_type: None,
            param_id: None,
        });
    }

    /// Push a parameter node with its ID for lookup
    pub(crate) fn push_param(
        &mut self,
        depth: usize,
        text: String,
        sections: Vec<DetailSectionData>,
        node_type: NodeType,
        param_id: u32,
    ) {
        self.nodes.push(TreeNode {
            depth,
            text,
            expanded: false,
            has_children: false,
            detail_sections: sections,
            node_type,
            section_type: None,
            service_list_type: None,
            param_id: Some(param_id),
        });
    }

    /// Push a service list section header with type information
    pub(crate) fn push_service_list_header(
        &mut self,
        depth: usize,
        text: String,
        expanded: bool,
        has_children: bool,
        sections: Vec<DetailSectionData>,
        service_list_type: ServiceListType,
    ) {
        self.nodes.push(TreeNode {
            depth,
            text,
            expanded,
            has_children,
            detail_sections: sections,
            node_type: NodeType::SectionHeader,
            section_type: None,
            service_list_type: Some(service_list_type),
            param_id: None,
        });
    }

    /// Push a top-level section header with type information
    pub(crate) fn push_section_header(
        &mut self,
        text: String,
        expanded: bool,
        has_children: bool,
        sections: Vec<DetailSectionData>,
        section_type: SectionType,
    ) {
        self.nodes.push(TreeNode {
            depth: 0,
            text,
            expanded,
            has_children,
            detail_sections: sections,
            node_type: NodeType::SectionHeader,
            section_type: Some(section_type),
            service_list_type: None,
            param_id: None,
        });
    }

    pub(crate) fn finish(self) -> Vec<TreeNode> {
        self.nodes
    }
}
