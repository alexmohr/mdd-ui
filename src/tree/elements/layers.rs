// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use cda_database::datatypes::{DiagLayer, ParentRef, Variant};

use super::variants::{
    com_params::add_com_params,
    dops::add_dops_section,
    functional_classes::add_functional_classes,
    parent_refs::add_parent_refs_with_details,
    placeholders::{add_additional_audiences, add_sdgs},
    requests::add_requests_section,
    responses::{add_neg_responses_section, add_pos_responses_section},
    services::add_diag_comms,
    state_charts::add_state_charts,
    tables::add_tables,
    unit_spec::add_unit_spec,
};
use crate::tree::builder::TreeBuilder;

/// Extension trait for adding DiagLayer structures to the tree
pub trait LayerExt {
    /// Add a complete diag layer with structured hierarchy for containers
    /// variant_parent_refs: Optional iterator to parent refs from the variant
    /// for fetching inherited services
    fn add_diag_layer_structured<'a>(
        &mut self,
        layer: &DiagLayer<'a>,
        depth: usize,
        layer_name: &str,
        _expand: bool,
        variant_parent_refs: Option<impl Iterator<Item = ParentRef<'a>> + 'a>,
        all_variants: Option<impl Iterator<Item = Variant<'a>> + 'a>,
    );
}

impl LayerExt for TreeBuilder {
    fn add_diag_layer_structured<'a>(
        &mut self,
        layer: &DiagLayer<'a>,
        depth: usize,
        layer_name: &str,
        _expand: bool,
        variant_parent_refs: Option<impl Iterator<Item = ParentRef<'a>> + 'a>,
        all_variants: Option<impl Iterator<Item = Variant<'a>> + 'a>,
    ) {
        // Collect parent refs into a vector so we can reuse them for multiple sections
        let parent_refs_vec: Option<Vec<ParentRef<'a>>> =
            variant_parent_refs.map(|iter| iter.collect());

        // Elements are ordered alphabetically for easier navigation

        // Additional Audiences
        add_additional_audiences(self, layer, depth);

        // ComParam Refs
        add_com_params(self, layer, depth);

        // Diag-Comms
        add_diag_comms(
            self,
            layer,
            depth,
            layer_name,
            parent_refs_vec.as_ref().map(|v| v.iter().cloned()),
        );

        // DOPs (Data Object Properties)
        add_dops_section(self, layer, depth);

        // Functional Classes - pass all variants so it can search across them
        add_functional_classes(self, layer, depth, all_variants);

        // Neg-Responses (from diag-comms) - use EXACTLY the same logic as DiagComm
        add_neg_responses_section(
            self,
            layer,
            depth,
            parent_refs_vec.as_ref().map(|v| v.iter().cloned()),
        );

        // Parent Refs
        add_parent_refs_with_details(
            self,
            layer,
            depth,
            parent_refs_vec.as_ref().map(|v| v.iter().cloned()),
        );

        // Pos-Responses (from diag-comms) - use EXACTLY the same logic as DiagComm
        add_pos_responses_section(
            self,
            layer,
            depth,
            parent_refs_vec.as_ref().map(|v| v.iter().cloned()),
        );

        // Requests (from diag-comms) - use EXACTLY the same logic as DiagComm
        add_requests_section(
            self,
            layer,
            depth,
            parent_refs_vec.as_ref().map(|v| v.iter().cloned()),
        );

        // SDGs
        add_sdgs(self, layer, depth);

        // State-Charts
        add_state_charts(self, layer, depth);

        // Tables (from parent refs)
        add_tables(
            self,
            depth,
            parent_refs_vec.as_ref().map(|v| v.iter().cloned()),
        );

        // Unit Spec (from ComParamRef -> ProtStack -> ComParamSubSet)
        add_unit_spec(self, layer, depth);
    }
}
