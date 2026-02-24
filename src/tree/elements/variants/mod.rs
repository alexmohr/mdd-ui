/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

// Submodules that represent the tree hierarchy under variants
pub mod com_params;
pub mod dops;
pub mod functional_classes;
pub mod parent_refs;
pub mod placeholders;
pub mod requests;
pub mod responses;
pub mod sdgs;
pub mod services;
pub mod state_charts;
pub mod tables;
pub mod unit_spec;

use cda_database::datatypes::{DiagLayer, DiagService, EcuDb, Variant as VariantWrap};

use super::layers::LayerExt;
use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ChildElementType, ColumnConstraint, DetailContent, DetailRow, DetailRowType,
        DetailSectionData, DetailSectionType, NodeType, RowMetadata, SectionType,
    },
};

/// Format a service display name from its diag_comm short_name, request_id,
/// and optional request_sub_function_id.
/// Returns `None` if `diag_comm()` is absent.
pub(crate) fn format_service_display_name(ds: &DiagService<'_>) -> Option<String> {
    let dc = ds.diag_comm()?;
    let name = dc.short_name().unwrap_or("?");

    let display_name = ds
        .request_id()
        .map(|sid| {
            ds.request_sub_function_id()
                .map(|(sub_fn, bit_len)| {
                    let sub_fn_str = if bit_len <= 8 {
                        format!("{sub_fn:02X}")
                    } else {
                        format!("{sub_fn:04X}")
                    };
                    format!("0x{:6} - {}", format!("{sid:02X}{sub_fn_str}"), name)
                })
                .unwrap_or_else(|| format!("0x{:6} - {}", format!("{sid:02X}"), name))
        })
        .unwrap_or_else(|| name.to_string());

    Some(display_name)
}

/// Format just the service ID portion (e.g., "0x2E01" or "0x22") without name.
/// Returns empty string if no request_id.
pub(crate) fn format_service_id(ds: &DiagService<'_>) -> String {
    ds.request_id()
        .map(|sid| {
            ds.request_sub_function_id()
                .map(|(sub_fn, bit_len)| {
                    let sub_fn_str = if bit_len <= 8 {
                        format!("{sub_fn:02X}")
                    } else {
                        format!("{sub_fn:04X}")
                    };
                    format!("0x{sid:02X}{sub_fn_str}")
                })
                .unwrap_or_else(|| format!("0x{sid:02X}"))
        })
        .unwrap_or_default()
}

/// Add all variants to the tree
pub fn add_variants(b: &mut TreeBuilder, ecu: &EcuDb<'_>) {
    // Add Variants section
    if let Some(variants) = ecu.variants() {
        // Collect all variants for cross-variant lookups (e.g., functional classes)
        let all_variants_vec: Vec<VariantWrap> = variants.iter().map(VariantWrap).collect();

        // Build variants overview table for the section header
        let variants_detail = build_variants_overview_table(&all_variants_vec);

        b.push_section_header(
            "Variants".to_string(),
            false,
            true,
            variants_detail,
            SectionType::Variants,
        );

        // Add each variant
        for (vi, variant) in variants.iter().enumerate() {
            let vw = VariantWrap(variant);
            let mut name = vw
                .diag_layer()
                .and_then(|l| l.short_name().map(str::to_owned))
                .unwrap_or_else(|| format!("variant_{vi}"));
            let is_base = vw.is_base_variant();

            // Add [base] suffix for base variants
            if is_base {
                name.push_str(" [base]");
            }

            // Build detail sections
            let mut detail_sections = vec![];

            // Add summary sections (overview and children)
            detail_sections.extend(build_variant_summary_section(&vw, &name));

            // Note: Parent refs are not shown in variant detail view per user request

            b.push_details_structured(
                1,
                name.clone(),
                is_base,
                true,
                detail_sections,
                NodeType::Container,
            );

            // Add diag layer content directly under variant (no section header)
            if let Some(dl) = vw.diag_layer() {
                let layer = DiagLayer(dl);
                // Pass parent refs from variant for inherited service lookup
                let parent_refs_iter = vw
                    .parent_refs()
                    .map(|pr| pr.iter().map(cda_database::datatypes::ParentRef));
                // Pass all variants for cross-variant lookups
                let all_variants_iter = all_variants_vec.iter().cloned();
                b.add_diag_layer_structured(
                    &layer,
                    2,
                    &name,
                    is_base,
                    parent_refs_iter,
                    Some(all_variants_iter),
                );
            }
        }
    }
}

/// Add all functional groups to the tree
pub fn add_functional_groups(b: &mut TreeBuilder, ecu: &EcuDb<'_>) {
    // Add functional groups as separate section
    if let Some(groups) = ecu.functional_groups()
        && !groups.is_empty()
    {
        b.push_section_header(
            "Functional Groups".to_string(),
            false,
            true,
            Vec::new(),
            SectionType::FunctionalGroups,
        );

        for fg in groups.iter() {
            if let Some(dl) = fg.diag_layer() {
                let layer = DiagLayer(dl);
                let name = layer.short_name().unwrap_or("unnamed");

                let detail_sections = build_layer_summary_section(&layer, name);

                b.push_details_structured(
                    1,
                    name.to_string(),
                    false,
                    true,
                    detail_sections,
                    NodeType::Container,
                );

                // Pass parent refs from functional group for inherited elements
                let parent_refs_iter = fg
                    .parent_refs()
                    .map(|pr| pr.iter().map(cda_database::datatypes::ParentRef));
                b.add_diag_layer_structured(
                    &layer,
                    2,
                    name,
                    false,
                    parent_refs_iter,
                    None::<std::iter::Empty<cda_database::datatypes::Variant>>,
                );
            }
        }
    }
}

/// Add all ECU shared data to the tree
pub fn add_ecu_shared_data(b: &mut TreeBuilder, ecu: &EcuDb<'_>) {
    // ECU shared data is accessed through functional groups -> parent refs
    // Following the pattern from the provided find_ecu_shared_services function

    // Collect ECU shared data from functional groups
    let ecu_shared_data_refs: Vec<_> = ecu
        .functional_groups()
        .into_iter()
        .flatten()
        .filter_map(|fg| {
            fg.parent_refs().and_then(|parent_refs| {
                // Find EcuSharedData parent refs
                let esd_refs: Vec<_> = parent_refs
                    .iter()
                    .filter_map(|parent_ref| {
                        let parent_ref = cda_database::datatypes::ParentRef(parent_ref);
                        match parent_ref.ref_type().try_into() {
                            Ok(cda_database::datatypes::ParentRefType::EcuSharedData) => {
                                parent_ref.ref__as_ecu_shared_data()
                            }
                            _ => None,
                        }
                    })
                    .collect();

                if esd_refs.is_empty() {
                    None
                } else {
                    Some(esd_refs)
                }
            })
        })
        .flatten()
        .collect();

    // Deduplicate by layer short name (same ECU shared data may be referenced by multiple FGs)
    let mut seen_names = std::collections::HashSet::new();
    let unique_esd: Vec<_> = ecu_shared_data_refs
        .into_iter()
        .filter(|esd| {
            if let Some(dl) = esd.diag_layer() {
                let name = dl.short_name().unwrap_or("");
                if !name.is_empty() && seen_names.contains(name) {
                    return false;
                }
                seen_names.insert(name.to_owned());
                true
            } else {
                false
            }
        })
        .collect();

    if !unique_esd.is_empty() {
        b.push_section_header(
            "ECU Shared Data".to_string(),
            false,
            true,
            Vec::new(),
            SectionType::EcuSharedData,
        );

        for esd in unique_esd.iter() {
            if let Some(dl) = esd.diag_layer() {
                let layer = DiagLayer(dl);
                let name = layer.short_name().unwrap_or("unnamed");

                let detail_sections = build_layer_summary_section(&layer, name);

                b.push_details_structured(
                    1,
                    name.to_string(),
                    false,
                    true,
                    detail_sections,
                    NodeType::Container,
                );

                // ECU shared data doesn't have parent refs like variants
                // add_diag_layer_structured will handle adding functional classes
                b.add_diag_layer_structured(
                    &layer,
                    2,
                    name,
                    false,
                    None::<std::iter::Empty<cda_database::datatypes::ParentRef>>,
                    None::<std::iter::Empty<cda_database::datatypes::Variant>>,
                );
            }
        }
    }
}

/// Add all protocols to the tree
pub fn add_protocols(b: &mut TreeBuilder, ecu: &EcuDb<'_>) {
    // Protocols are accessed through functional groups -> parent refs
    // Similar to ECU shared data

    // Collect protocols from functional groups
    let protocol_refs: Vec<_> = ecu
        .functional_groups()
        .into_iter()
        .flatten()
        .filter_map(|fg| {
            fg.parent_refs().and_then(|parent_refs| {
                // Find Protocol parent refs
                let proto_refs: Vec<_> = parent_refs
                    .iter()
                    .filter_map(|parent_ref| {
                        let parent_ref = cda_database::datatypes::ParentRef(parent_ref);
                        match parent_ref.ref_type().try_into() {
                            Ok(cda_database::datatypes::ParentRefType::Protocol) => {
                                parent_ref.ref__as_protocol()
                            }
                            _ => None,
                        }
                    })
                    .collect();

                if proto_refs.is_empty() {
                    None
                } else {
                    Some(proto_refs)
                }
            })
        })
        .flatten()
        .collect();

    // Deduplicate by layer short name
    let mut seen_names = std::collections::HashSet::new();
    let unique_protocols: Vec<_> = protocol_refs
        .into_iter()
        .filter(|protocol| {
            protocol.diag_layer().is_some_and(|dl| {
                let name = dl.short_name().unwrap_or("");
                if !name.is_empty() && seen_names.contains(name) {
                    return false;
                }
                seen_names.insert(name.to_owned());
                true
            })
        })
        .collect();

    if !unique_protocols.is_empty() {
        b.push_section_header(
            "Protocols".to_string(),
            false,
            true,
            Vec::new(),
            SectionType::Protocols,
        );

        for protocol_wrap in unique_protocols.iter() {
            let Some(dl) = protocol_wrap.diag_layer() else {
                continue;
            };
            let layer = DiagLayer(dl);
            let name = layer.short_name().unwrap_or("unnamed");

            b.push(1, name.to_string(), false, true, NodeType::Container);

            // For protocols, pass None for parent_refs since DOPs
            // come from the protocol's own ComParamSpec
            // The add_dops_section function already handles collecting DOPs from protocols
            b.add_diag_layer_structured(
                &layer,
                2,
                name,
                false,
                None::<std::iter::Empty<cda_database::datatypes::ParentRef>>,
                None::<std::iter::Empty<cda_database::datatypes::Variant>>,
            );
        }
    }
}

/// Build variant summary section with info and children table
fn build_variant_summary_section(vw: &VariantWrap<'_>, name: &str) -> Vec<DetailSectionData> {
    let mut sections = vec![];

    // Create info table section
    let header = DetailRow::header(
        vec!["Property".to_owned(), "Value".to_owned()],
        vec![CellType::Text, CellType::Text],
    );

    let mut info_rows = vec![
        DetailRow::normal(
            vec!["Variant".to_owned(), name.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
        DetailRow::normal(
            vec!["Base Variant".to_owned(), vw.is_base_variant().to_string()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
    ];

    if let Some(dl) = vw.diag_layer() {
        let layer = DiagLayer(dl);
        append_layer_info_rows(&layer, &mut info_rows);
    }

    sections.push(
        DetailSectionData::new(
            "Overview".to_owned(),
            DetailContent::Table {
                header,
                rows: info_rows,
                constraints: vec![
                    ColumnConstraint::Percentage(30),
                    ColumnConstraint::Percentage(70),
                ],
                use_row_selection: true,
            },
            false,
        )
        .with_type(DetailSectionType::Overview),
    );

    sections
}

/// Build summary section for a DiagLayer (used by functional groups and ECU shared data)
fn build_layer_summary_section(layer: &DiagLayer<'_>, name: &str) -> Vec<DetailSectionData> {
    let header = DetailRow::header(
        vec!["Property".to_owned(), "Value".to_owned()],
        vec![CellType::Text, CellType::Text],
    );

    let mut info_rows = vec![DetailRow::normal(
        vec!["Name".to_owned(), name.to_owned()],
        vec![CellType::Text, CellType::Text],
        0,
    )];

    append_layer_info_rows(layer, &mut info_rows);

    vec![
        DetailSectionData::new(
            "Overview".to_owned(),
            DetailContent::Table {
                header,
                rows: info_rows,
                constraints: vec![
                    ColumnConstraint::Percentage(30),
                    ColumnConstraint::Percentage(70),
                ],
                use_row_selection: true,
            },
            false,
        )
        .with_type(DetailSectionType::Overview),
    ]
}

/// Append DiagLayer info rows (short name, long name, children) to an existing row list
fn append_layer_info_rows(layer: &DiagLayer<'_>, info_rows: &mut Vec<DetailRow>) {
    if let Some(sn) = layer.short_name() {
        info_rows.push(DetailRow::normal(
            vec!["Short Name".to_owned(), sn.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ));
    }
    if let Some(ln) = layer.long_name() {
        info_rows.push(DetailRow::normal(
            vec!["Long Name".to_owned(), format!("{:?}", ln)],
            vec![CellType::Text, CellType::Text],
            0,
        ));
    }

    if let Some(children_rows) = build_children_rows(layer) {
        info_rows.push(DetailRow::normal(
            vec!["".to_owned(), "".to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ));
        info_rows.push(DetailRow::normal(
            vec!["Children".to_owned(), "".to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ));
        info_rows.extend(children_rows);
    }
}

/// Build rows listing all child elements with counts
fn build_children_rows(layer: &DiagLayer<'_>) -> Option<Vec<DetailRow>> {
    let mut rows = Vec::new();

    // Check each child element type and add to table if present

    // ComParam Refs
    if let Some(cp_refs) = layer.com_param_refs()
        && !cp_refs.is_empty()
    {
        rows.push(DetailRow {
            cells: vec!["ComParam Refs".to_owned(), cp_refs.len().to_string()],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            row_type: DetailRowType::ChildElement,
            metadata: Some(RowMetadata::ChildElement {
                element_type: ChildElementType::ComParamRefs,
            }),
        });
    }

    // Diag-Comms (Services + Jobs)
    let service_count = layer.diag_services().map(|s| s.len()).unwrap_or(0);
    let job_count = layer.single_ecu_jobs().map(|j| j.len()).unwrap_or(0);
    if service_count + job_count > 0 {
        rows.push(DetailRow {
            cells: vec![
                "Diag-Comms".to_owned(),
                format!("{} services, {} jobs", service_count, job_count),
            ],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            row_type: DetailRowType::ChildElement,
            metadata: Some(RowMetadata::ChildElement {
                element_type: ChildElementType::DiagComms,
            }),
        });
    }

    // Functional Classes
    if let Some(fcs) = layer.funct_classes()
        && !fcs.is_empty()
    {
        rows.push(DetailRow {
            cells: vec!["Functional Classes".to_owned(), fcs.len().to_string()],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            row_type: DetailRowType::ChildElement,
            metadata: Some(RowMetadata::ChildElement {
                element_type: ChildElementType::FunctionalClasses,
            }),
        });
    }

    // Neg-Responses
    let neg_count: usize = layer
        .diag_services()
        .map(|services| {
            services
                .iter()
                .filter_map(|s| {
                    cda_database::datatypes::DiagService(s)
                        .neg_responses()
                        .map(|r| r.len())
                })
                .sum()
        })
        .unwrap_or(0);
    if neg_count > 0 {
        rows.push(DetailRow {
            cells: vec!["Neg-Responses".to_owned(), neg_count.to_string()],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            row_type: DetailRowType::ChildElement,
            metadata: Some(RowMetadata::ChildElement {
                element_type: ChildElementType::NegResponses,
            }),
        });
    }

    // Pos-Responses
    let pos_count: usize = layer
        .diag_services()
        .map(|services| {
            services
                .iter()
                .filter_map(|s| {
                    cda_database::datatypes::DiagService(s)
                        .pos_responses()
                        .map(|r| r.len())
                })
                .sum()
        })
        .unwrap_or(0);
    if pos_count > 0 {
        rows.push(DetailRow {
            cells: vec!["Pos-Responses".to_owned(), pos_count.to_string()],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            row_type: DetailRowType::ChildElement,
            metadata: Some(RowMetadata::ChildElement {
                element_type: ChildElementType::PosResponses,
            }),
        });
    }

    // Requests
    let request_count = layer
        .diag_services()
        .map(|services| {
            services
                .iter()
                .filter(|&s| cda_database::datatypes::DiagService(s).request().is_some())
                .count()
        })
        .unwrap_or(0);
    if request_count > 0 {
        rows.push(DetailRow {
            cells: vec!["Requests".to_owned(), request_count.to_string()],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            row_type: DetailRowType::ChildElement,
            metadata: Some(RowMetadata::ChildElement {
                element_type: ChildElementType::Requests,
            }),
        });
    }

    // SDGs
    let sdg_count = layer
        .sdgs()
        .and_then(|sdgs| sdgs.sdgs())
        .filter(|list| !list.is_empty())
        .map(|list| list.len());
    if let Some(count) = sdg_count {
        rows.push(DetailRow {
            cells: vec!["SDGs".to_owned(), count.to_string()],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            row_type: DetailRowType::ChildElement,
            metadata: Some(RowMetadata::ChildElement {
                element_type: ChildElementType::SDGs,
            }),
        });
    }

    // State-Charts
    rows.extend(
        layer
            .state_charts()
            .filter(|c| !c.is_empty())
            .map(|charts| DetailRow {
                cells: vec!["State-Charts".to_owned(), charts.len().to_string()],
                cell_types: vec![CellType::Text, CellType::Text],
                indent: 0,
                row_type: DetailRowType::ChildElement,
                metadata: Some(RowMetadata::ChildElement {
                    element_type: ChildElementType::StateCharts,
                }),
            }),
    );

    if rows.is_empty() {
        return None;
    }

    Some(rows)
}

fn build_variants_overview_table(variants: &[VariantWrap]) -> Vec<DetailSectionData> {
    if variants.is_empty() {
        return vec![];
    }

    // Build table header
    let header = DetailRow::header(
        vec!["Name".to_owned(), "Is Base".to_owned()],
        vec![CellType::Text, CellType::Text],
    );

    let mut rows = Vec::new();

    // Add each variant to the table
    for variant in variants {
        let name = variant
            .diag_layer()
            .and_then(|l| l.short_name())
            .unwrap_or("unnamed")
            .to_owned();

        let is_base = if variant.is_base_variant() {
            "Yes"
        } else {
            "No"
        };

        rows.push(DetailRow::normal(
            vec![name, is_base.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ));
    }

    vec![
        DetailSectionData::new(
            "Variants Overview".to_owned(),
            DetailContent::Table {
                header,
                rows,
                constraints: vec![
                    ColumnConstraint::Percentage(70),
                    ColumnConstraint::Percentage(30),
                ],
                use_row_selection: true,
            },
            false,
        )
        .with_type(DetailSectionType::Overview),
    ]
}
