/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::{DiagLayer, DiagService, Parameter, ParentRef};

use super::{
    params::{build_param_detail_sections, build_param_section, build_service_list_table_section},
    services::{build_service_overview_section, get_parent_ref_services_recursive},
};
use crate::tree::{
    builder::TreeBuilder,
    types::{DetailContent, DetailSectionData, DetailSectionType, NodeType},
};

/// Add requests section to the tree
/// This uses EXACTLY the same logic and display as `DiagComm` - just filtered
/// to show only services with requests
pub fn add_requests_section<'a>(
    b: &mut TreeBuilder,
    layer: &DiagLayer<'a>,
    depth: usize,
    variant_parent_refs: Option<impl Iterator<Item = ParentRef<'a>> + 'a>,
) {
    // Collect own services that have requests
    let own_services: Vec<DiagService<'_>> = layer
        .diag_services()
        .map(|services| {
            services
                .iter()
                .map(DiagService)
                .filter(|ds| ds.request().is_some())
                .collect()
        })
        .unwrap_or_default();

    // Collect services from parent refs with source layer names (that have requests)
    let parent_services: Vec<(DiagService<'_>, String)> =
        if let Some(parent_refs) = variant_parent_refs {
            get_parent_ref_services_recursive(parent_refs)
                .into_iter()
                .filter(|(ds, _)| ds.request().is_some())
                .collect()
        } else {
            Vec::new()
        };

    let total_count = own_services.len().saturating_add(parent_services.len());

    if total_count > 0 {
        // Push header first (empty details — patched below with indices).
        let header_idx = b.next_index();
        b.push_service_list_header(
            depth,
            format!("Requests ({total_count})"),
            false,
            true,
            vec![],
            crate::tree::ServiceListType::Requests,
        );

        // Push children, collecting (short_name → tree index).
        let mut node_indices = std::collections::HashMap::new();

        let all_services: Vec<(&DiagService<'_>, Option<&str>)> = own_services
            .iter()
            .map(|ds| (ds, None))
            .chain(
                parent_services
                    .iter()
                    .map(|(ds, name)| (ds, Some(name.as_str()))),
            )
            .collect();

        for (ds, source_layer) in all_services {
            let Some(display_name) = super::format_service_display_name(ds) else {
                continue;
            };

            let short_name = ds
                .diag_comm()
                .and_then(|dc| dc.short_name())
                .unwrap_or("?")
                .to_owned();
            node_indices.insert(short_name.clone(), b.next_index());
            let container_idx = source_layer.and_then(|name| b.find_container_index(name));
            let sections = build_request_view_sections(ds, source_layer, container_idx);

            let has_params = ds
                .request()
                .and_then(|req| req.params())
                .is_some_and(|p| !p.is_empty());

            b.push_service_node(
                depth.saturating_add(1),
                display_name.clone(),
                false,
                has_params,
                sections,
                NodeType::Request,
                short_name,
            );

            ds.request()
                .and_then(|req| req.params())
                .into_iter()
                .flat_map(|params| params.iter().map(Parameter))
                .for_each(|param| {
                    let param_name = param.short_name().unwrap_or("?").to_owned();
                    let param_detail = build_param_detail_sections(&param);
                    let param_id = param.id();

                    b.push_param(
                        depth.saturating_add(2),
                        param_name,
                        param_detail,
                        NodeType::Default,
                        param_id,
                    );
                });
        }

        // Build table with tree-node indices and patch the header.
        let detail_section = build_service_list_table_section(
            &own_services,
            &parent_services,
            "Requests",
            DetailSectionType::Requests,
            &node_indices,
        );
        b.set_detail_sections(header_idx, vec![detail_section]);
    }
}

/// Build the Request tab section - this is the core rendering logic for Request data
/// `DiagComm` module should import and use this function to render the Request tab
/// Always returns a section (empty table if no request data)
pub fn build_request_section(ds: &DiagService<'_>) -> DetailSectionData {
    let params: Vec<Parameter<'_>> = ds
        .request()
        .and_then(|req| req.params())
        .into_iter()
        .flatten()
        .map(Parameter)
        .collect();

    build_param_section("Request", params, DetailSectionType::Requests)
}

/// Build complete service view with Request tab (used by Requests section)
fn build_request_view_sections(
    ds: &DiagService<'_>,
    parent_layer_name: Option<&str>,
    container_index: Option<usize>,
) -> Vec<DetailSectionData> {
    let mut sections = Vec::new();

    let service_name = ds.diag_comm().and_then(|dc| dc.short_name()).unwrap_or("?");
    let id_str = super::format_service_id(ds);
    let header_title = if id_str.is_empty() {
        format!("Request - {service_name}")
    } else {
        format!("Request - {id_str} - {service_name}")
    };

    sections.push(DetailSectionData {
        title: header_title,
        render_as_header: true,
        section_type: DetailSectionType::Header,
        content: DetailContent::PlainText(vec![]),
    });
    sections.push(build_service_overview_section(
        ds,
        parent_layer_name,
        container_index,
    ));
    sections.push(build_request_section(ds));

    sections
}
