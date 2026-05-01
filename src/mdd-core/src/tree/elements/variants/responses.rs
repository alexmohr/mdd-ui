/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::{DiagLayer, DiagService, Parameter, ParentRef};

use super::{
    format_service_display_name, format_service_id,
    params::{build_param_detail_sections, build_param_section, build_service_list_table_section},
    services::{build_service_overview_section, get_parent_ref_services_recursive},
};
use crate::tree::{
    builder::TreeBuilder,
    types::{DetailContent, DetailSectionData, DetailSectionType, NodeType, ServiceListType},
};

/// Configuration for building response sections, capturing the difference
/// between positive and negative responses.
struct ResponseKind {
    label: &'static str,
    section_type: DetailSectionType,
    node_type: NodeType,
    service_list_type: ServiceListType,
}

const POS_RESPONSE: ResponseKind = ResponseKind {
    label: "Pos-Response",
    section_type: DetailSectionType::PosResponses,
    node_type: NodeType::PosResponse,
    service_list_type: ServiceListType::PosResponses,
};

const NEG_RESPONSE: ResponseKind = ResponseKind {
    label: "Neg-Response",
    section_type: DetailSectionType::NegResponses,
    node_type: NodeType::NegResponse,
    service_list_type: ServiceListType::NegResponses,
};

/// Dispatch to the correct response accessor based on `section_type`.
/// This avoids spelling out the unnameable flatbuffers return type.
macro_rules! responses_of {
    ($ds:expr, $kind:expr) => {
        match $kind.section_type {
            DetailSectionType::PosResponses => $ds.pos_responses(),
            DetailSectionType::NegResponses => $ds.neg_responses(),
            DetailSectionType::Header
            | DetailSectionType::Overview
            | DetailSectionType::Services
            | DetailSectionType::Requests
            | DetailSectionType::ComParams
            | DetailSectionType::States
            | DetailSectionType::RelatedRefs
            | DetailSectionType::FunctionalClass
            | DetailSectionType::NotInheritedDiagComms
            | DetailSectionType::NotInheritedDops
            | DetailSectionType::NotInheritedTables
            | DetailSectionType::NotInheritedVariables
            | DetailSectionType::EcuLocks
            | DetailSectionType::Custom => None,
        }
    };
}

/// Build response sections for a given kind (pos or neg).
/// Always returns at least one section (empty table if no response data).
fn build_responses_sections(ds: &DiagService<'_>, kind: &ResponseKind) -> Vec<DetailSectionData> {
    let sections: Vec<DetailSectionData> = responses_of!(ds, kind)
        .into_iter()
        .flat_map(|responses| responses.iter().enumerate())
        .map(|(i, resp)| {
            let params = resp.params().into_iter().flatten().map(Parameter);
            build_param_section(
                &format!("{} {}", kind.label, i.saturating_add(1)),
                params,
                kind.section_type,
            )
        })
        .collect();

    if sections.is_empty() {
        vec![build_param_section(
            kind.label,
            std::iter::empty(),
            kind.section_type,
        )]
    } else {
        sections
    }
}

/// Build Pos-Response sections
pub fn build_pos_responses_sections(ds: &DiagService<'_>) -> Vec<DetailSectionData> {
    build_responses_sections(ds, &POS_RESPONSE)
}

/// Build Neg-Response sections
pub fn build_neg_responses_sections(ds: &DiagService<'_>) -> Vec<DetailSectionData> {
    build_responses_sections(ds, &NEG_RESPONSE)
}

/// Add a single service with responses to the tree
fn add_response_service(
    b: &mut TreeBuilder,
    ds: &DiagService<'_>,
    depth: usize,
    source_layer: Option<&str>,
    kind: &ResponseKind,
    container_index: Option<usize>,
) {
    let Some(display_name) = format_service_display_name(ds) else {
        return;
    };

    let short_name = ds
        .diag_comm()
        .and_then(|dc| dc.short_name())
        .unwrap_or("?")
        .to_owned();
    let sections =
        build_response_view_sections(ds, b.ecu_name(), source_layer, kind, container_index);
    let has_params = responses_of!(ds, kind).is_some_and(|r| {
        r.iter()
            .any(|resp| resp.params().is_some_and(|p| !p.is_empty()))
    });

    b.push_service_node(
        depth.saturating_add(1),
        display_name,
        false,
        has_params,
        sections,
        kind.node_type,
        short_name,
    );

    let response_count = responses_of!(ds, kind).map_or(0, |r| r.len());
    for (resp_idx, resp) in responses_of!(ds, kind)
        .into_iter()
        .flat_map(|r| r.iter().enumerate())
    {
        let Some(params) = resp.params().filter(|p| !p.is_empty()) else {
            continue;
        };
        if response_count > 1 {
            b.push_details_structured(
                depth.saturating_add(2),
                format!("Response {}", resp_idx.saturating_add(1)),
                false,
                true,
                vec![],
                NodeType::Default,
            );
        }
        let base_depth = if response_count > 1 {
            depth.saturating_add(3)
        } else {
            depth.saturating_add(2)
        };
        for param in params.iter().map(Parameter) {
            let param_name = param.short_name().unwrap_or("?").to_owned();
            let param_detail = build_param_detail_sections(&param);
            b.push_param(
                base_depth,
                param_name.clone(),
                param_name,
                param_detail,
                NodeType::Default,
                param.id(),
            );
        }
    }
}

/// Add a responses section (pos or neg) to the tree
fn add_responses_section<'a>(
    b: &mut TreeBuilder,
    layer: &DiagLayer<'a>,
    depth: usize,
    variant_parent_refs: Option<impl Iterator<Item = ParentRef<'a>> + 'a>,
    kind: &ResponseKind,
) {
    let has_responses =
        |ds: &DiagService<'_>| -> bool { responses_of!(ds, kind).is_some_and(|r| !r.is_empty()) };

    let own_services: Vec<DiagService<'_>> = layer
        .diag_services()
        .map(|services| {
            services
                .iter()
                .map(DiagService)
                .filter(|ds| has_responses(ds))
                .collect()
        })
        .unwrap_or_default();

    let parent_services: Vec<(DiagService<'_>, String)> =
        if let Some(parent_refs) = variant_parent_refs {
            get_parent_ref_services_recursive(parent_refs)
                .into_iter()
                .filter(|(ds, _)| has_responses(ds))
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
            format!("{}s ({total_count})", kind.label),
            false,
            true,
            vec![],
            kind.service_list_type,
        );

        // Push children, collecting (short_name → tree index).
        let mut node_indices = std::collections::HashMap::new();

        for ds in &own_services {
            if ds.diag_comm().is_some() {
                let sn = ds
                    .diag_comm()
                    .and_then(|dc| dc.short_name())
                    .unwrap_or("?")
                    .to_owned();
                node_indices.insert(sn, b.next_index());
            }
            add_response_service(b, ds, depth, None, kind, None);
        }

        for (ds, source_layer_name) in &parent_services {
            if ds.diag_comm().is_some() {
                let sn = ds
                    .diag_comm()
                    .and_then(|dc| dc.short_name())
                    .unwrap_or("?")
                    .to_owned();
                node_indices.insert(sn, b.next_index());
            }
            let container_idx = b.find_container_index(source_layer_name);
            add_response_service(
                b,
                ds,
                depth,
                Some(source_layer_name.as_str()),
                kind,
                container_idx,
            );
        }

        // Build table with tree-node indices and patch the header.
        let detail_section = build_service_list_table_section(
            &own_services,
            &parent_services,
            &format!("{}s", kind.label),
            kind.section_type,
            &node_indices,
        );
        b.set_detail_sections(header_idx, vec![detail_section]);
    }
}

/// Add positive responses section to the tree
pub fn add_pos_responses_section<'a>(
    b: &mut TreeBuilder,
    layer: &DiagLayer<'a>,
    depth: usize,
    variant_parent_refs: Option<impl Iterator<Item = ParentRef<'a>> + 'a>,
) {
    add_responses_section(b, layer, depth, variant_parent_refs, &POS_RESPONSE);
}

/// Add negative responses section to the tree
pub fn add_neg_responses_section<'a>(
    b: &mut TreeBuilder,
    layer: &DiagLayer<'a>,
    depth: usize,
    variant_parent_refs: Option<impl Iterator<Item = ParentRef<'a>> + 'a>,
) {
    add_responses_section(b, layer, depth, variant_parent_refs, &NEG_RESPONSE);
}

/// Build complete service view with response tabs
fn build_response_view_sections(
    ds: &DiagService<'_>,
    ecu_name: &str,
    parent_layer_name: Option<&str>,
    kind: &ResponseKind,
    container_index: Option<usize>,
) -> Vec<DetailSectionData> {
    let mut sections = Vec::new();

    let service_name = ds.diag_comm().and_then(|dc| dc.short_name()).unwrap_or("?");
    let sid = format_service_id(ds);
    let label = kind.label.replace('-', " ");
    let header_title = if sid.is_empty() {
        format!("{label} - {service_name}")
    } else {
        format!("{label} - {sid} - {service_name}")
    };

    sections.push(DetailSectionData {
        title: header_title,
        render_as_header: true,
        section_type: DetailSectionType::Header,
        content: DetailContent::PlainText(vec![]),
        byte_pattern_rows: None,
    });

    sections.push(build_service_overview_section(
        ds,
        ecu_name,
        parent_layer_name,
        container_index,
    ));
    sections.extend(build_responses_sections(ds, kind));

    sections
}
