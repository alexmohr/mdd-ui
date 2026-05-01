/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use std::sync::Arc;

use super::{
    resolve_all_indices,
    types::{DetailSectionData, NodePayload, NodeType, SectionType, ServiceListType, TreeNode},
};

/// Configuration for a single tree node, used to avoid repeating the full
/// `TreeNode` struct literal in every `push_*` method.
#[derive(Default)]
struct NodeConfig {
    depth: usize,
    text: String,
    expanded: bool,
    has_children: bool,
    sections: Vec<DetailSectionData>,
    node_type: NodeType,
    payload: NodePayload,
}

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

    fn push_node(&mut self, cfg: NodeConfig) {
        self.nodes.push(TreeNode {
            depth: cfg.depth,
            text: cfg.text,
            expanded: cfg.expanded,
            has_children: cfg.has_children,
            detail_sections: Arc::from(cfg.sections),
            node_type: cfg.node_type,
            payload: cfg.payload,
            parent_idx: None,
            diff_status: None,
        });
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
        self.push_node(NodeConfig {
            depth,
            text,
            expanded,
            has_children,
            sections,
            node_type,
            ..NodeConfig::default()
        });
    }

    /// Push a functional-class node with its canonical short name.
    pub(crate) fn push_functional_class(
        &mut self,
        depth: usize,
        short_name: String,
        text: String,
        sections: Vec<DetailSectionData>,
    ) {
        self.push_node(NodeConfig {
            depth,
            text,
            sections,
            node_type: NodeType::FunctionalClass,
            payload: NodePayload::FunctionalClass { short_name },
            ..NodeConfig::default()
        });
    }

    /// Push a parameter node with its ID and canonical short name for lookup.
    pub(crate) fn push_param(
        &mut self,
        depth: usize,
        short_name: String,
        text: String,
        sections: Vec<DetailSectionData>,
        node_type: NodeType,
        param_id: u32,
    ) {
        self.push_node(NodeConfig {
            depth,
            text,
            sections,
            node_type,
            payload: NodePayload::Parameter {
                param_id,
                short_name,
            },
            ..NodeConfig::default()
        });
    }

    /// Push a container node with parent ref container names from the database.
    /// `parent_ref_names` stores the short names of all parent-ref containers
    /// so the navigation system can walk the DB inheritance chain.
    pub(crate) fn push_container(
        &mut self,
        depth: usize,
        short_name: String,
        text: String,
        sections: Vec<DetailSectionData>,
        parent_ref_names: Vec<String>,
    ) {
        self.push_node(NodeConfig {
            depth,
            text,
            has_children: true,
            sections,
            node_type: NodeType::Container,
            payload: NodePayload::Container {
                short_name,
                parent_ref_names,
                parent_ref_indices: Vec::new(),
            },
            ..NodeConfig::default()
        });
    }

    /// Push a diagcomm node (Service, `ParentRefService`, Request, Response, Job)
    /// with its canonical `short_name` stored for type-safe lookups.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push_service_node(
        &mut self,
        depth: usize,
        text: String,
        expanded: bool,
        has_children: bool,
        sections: Vec<DetailSectionData>,
        node_type: NodeType,
        service_short_name: String,
    ) {
        self.push_node(NodeConfig {
            depth,
            text,
            expanded,
            has_children,
            sections,
            node_type,
            payload: NodePayload::DiagComm { service_short_name },
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
        self.push_node(NodeConfig {
            depth,
            text,
            expanded,
            has_children,
            sections,
            node_type: NodeType::SectionHeader,
            payload: NodePayload::ServiceListHeader { service_list_type },
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
        self.push_node(NodeConfig {
            text,
            expanded,
            has_children,
            sections,
            node_type: NodeType::SectionHeader,
            payload: NodePayload::SectionHeader { section_type },
            ..NodeConfig::default()
        });
    }

    /// Returns the index that the next pushed node will receive.
    pub(crate) fn next_index(&self) -> usize {
        self.nodes.len()
    }

    /// Replace the detail sections of a previously pushed node.
    ///
    /// Used to patch a service-list header after its children have been
    /// pushed, so the header table can store direct tree-node indices.
    pub(crate) fn set_detail_sections(
        &mut self,
        node_idx: usize,
        sections: Vec<DetailSectionData>,
    ) {
        if let Some(node) = self.nodes.get_mut(node_idx) {
            node.detail_sections = Arc::from(sections);
        }
    }

    /// Find a `Container` node whose `short_name` matches `name`.
    /// Returns the tree index, or `None` if not found.
    pub(crate) fn find_container_index(&self, name: &str) -> Option<usize> {
        self.nodes.iter().position(|n| {
            matches!(n.node_type, NodeType::Container)
                && n.short_name().is_some_and(|sn| sn == name)
        })
    }

    pub(crate) fn finish(mut self) -> Vec<TreeNode> {
        Self::compute_parent_indices(&mut self.nodes);
        resolve_all_indices(&mut self.nodes);
        self.nodes
    }

    /// Populate `parent_idx` for every node by tracking the most recent
    /// ancestor at each depth level during a single forward pass.
    fn compute_parent_indices(nodes: &mut [TreeNode]) {
        // parent_at_depth[d] = index of the most recent node at depth d.
        let max_depth = nodes.iter().map(|n| n.depth).max().unwrap_or(0);
        let mut parent_at_depth = vec![0usize; max_depth.saturating_add(1)];

        for i in 0..nodes.len() {
            let depth = nodes.get(i).map_or(0, |n| n.depth);
            if let Some(slot) = parent_at_depth.get_mut(depth) {
                *slot = i;
            }
            if depth > 0 {
                let parent = parent_at_depth.get(depth.saturating_sub(1)).copied();
                if let Some(n) = nodes.get_mut(i) {
                    n.parent_idx = parent;
                }
            }
        }
    }
}
