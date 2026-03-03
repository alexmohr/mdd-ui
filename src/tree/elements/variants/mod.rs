/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

// Submodules that represent the tree hierarchy under variants
pub mod com_params;
pub mod dops;
pub mod functional_classes;
pub mod params;
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
use parent_refs::{build_parent_refs_detail_section, extract_parent_ref_short_names};

use super::layers::LayerExt;
use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellJumpTarget, CellType, ChildElementType, ColumnConstraint, DetailContent, DetailRow,
        DetailRowType, DetailSectionData, DetailSectionType, RowMetadata, SectionType,
    },
};

/// Format a service display name from its `diag_comm` `short_name`, `request_id`,
/// and optional `request_sub_function_id`.
/// Returns `None` if `diag_comm()` is absent.
pub(crate) fn format_service_display_name(ds: &DiagService<'_>) -> Option<String> {
    let dc = ds.diag_comm()?;
    let name = dc.short_name().unwrap_or("?");

    let display_name = ds.request_id().map_or_else(
        || name.to_string(),
        |sid| {
            ds.request_sub_function_id().map_or_else(
                || {
                    let sid_hex = format!("{sid:02X}");
                    format!("0x{sid_hex:6} - {name}")
                },
                |(sub_fn, bit_len)| {
                    let sub_fn_str = if bit_len <= 8 {
                        format!("{sub_fn:02X}")
                    } else {
                        format!("{sub_fn:04X}")
                    };
                    let full_id = format!("{sid:02X}{sub_fn_str}");
                    format!("0x{full_id:6} - {name}")
                },
            )
        },
    );

    Some(display_name)
}

/// Format just the service ID portion (e.g., "0x2E01" or "0x22") without name.
/// Returns empty string if no `request_id`.
pub(crate) fn format_service_id(ds: &DiagService<'_>) -> String {
    ds.request_id().map_or_else(String::new, |sid| {
        ds.request_sub_function_id().map_or_else(
            || format!("0x{sid:02X}"),
            |(sub_fn, bit_len)| {
                let sub_fn_str = if bit_len <= 8 {
                    format!("{sub_fn:02X}")
                } else {
                    format!("{sub_fn:04X}")
                };
                format!("0x{sid:02X}{sub_fn_str}")
            },
        )
    })
}

/// Add all variants to the tree
pub fn add_variants(b: &mut TreeBuilder, ecu: &EcuDb<'_>) {
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

            let mut detail_sections = vec![];

            detail_sections.extend(build_variant_summary_section(&vw, &name));

            // Add parent refs section if present
            if let Some(parent_refs_section) = build_parent_refs_detail_section(
                vw.parent_refs()
                    .map(|pr| pr.iter().map(cda_database::datatypes::ParentRef)),
            ) {
                detail_sections.push(parent_refs_section);
            }

            // Extract parent ref container names from the DB for hierarchy navigation
            let parent_ref_names = extract_parent_ref_short_names(
                vw.parent_refs()
                    .map(|pr| pr.iter().map(cda_database::datatypes::ParentRef)),
            );

            b.push_container(1, name.clone(), detail_sections, parent_ref_names);

            // Add diag layer content directly under variant (no section header)
            if let Some(dl) = vw.diag_layer() {
                let layer = DiagLayer(dl);
                // Pass parent refs from variant for inherited service lookup
                let parent_refs_iter = vw
                    .parent_refs()
                    .map(|pr| pr.iter().map(cda_database::datatypes::ParentRef));
                // Pass all variants for cross-variant lookups
                let all_variants_iter = all_variants_vec.iter().cloned();
                b.add_diag_layer_structured(&layer, 2, parent_refs_iter, Some(all_variants_iter));
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
        let names: Vec<String> = groups
            .iter()
            .filter_map(|fg| {
                fg.diag_layer()
                    .and_then(|dl| dl.short_name().map(str::to_owned))
            })
            .collect();
        let overview = build_names_overview_table(&names, "Functional Groups Overview");

        b.push_section_header(
            "Functional Groups".to_string(),
            false,
            true,
            overview,
            SectionType::FunctionalGroups,
        );

        for fg in groups {
            if let Some(dl) = fg.diag_layer() {
                let layer = DiagLayer(dl);
                let name = layer.short_name().unwrap_or("unnamed");

                let mut detail_sections = build_layer_summary_section(&layer, name);

                // Add parent refs section if present
                if let Some(parent_refs_section) = build_parent_refs_detail_section(
                    fg.parent_refs()
                        .map(|pr| pr.iter().map(cda_database::datatypes::ParentRef)),
                ) {
                    detail_sections.push(parent_refs_section);
                }

                // Extract parent ref container names from the DB for hierarchy navigation
                let parent_ref_names = extract_parent_ref_short_names(
                    fg.parent_refs()
                        .map(|pr| pr.iter().map(cda_database::datatypes::ParentRef)),
                );

                b.push_container(1, name.to_string(), detail_sections, parent_ref_names);

                // Pass parent refs from functional group for inherited elements
                let parent_refs_iter = fg
                    .parent_refs()
                    .map(|pr| pr.iter().map(cda_database::datatypes::ParentRef));
                b.add_diag_layer_structured(
                    &layer,
                    2,
                    parent_refs_iter,
                    None::<std::iter::Empty<cda_database::datatypes::Variant>>,
                );
            }
        }
    }
}

/// Collect `DiagLayer`s reachable via variant or functional-group parent refs,
/// using `get_layer` to extract the inner layer for a chosen parent-ref type
/// (all other types should return `None`). Deduplicates by short name.
fn collect_parent_ref_layers<'a>(
    ecu: &EcuDb<'a>,
    get_layer: impl Fn(cda_database::datatypes::ParentRef<'a>) -> Option<DiagLayer<'a>>,
) -> Vec<DiagLayer<'a>> {
    let mut all: Vec<DiagLayer<'a>> = Vec::new();

    // Collect from variants
    if let Some(variants) = ecu.variants() {
        let variant_layers: Vec<DiagLayer<'a>> = variants
            .iter()
            .filter_map(|v| {
                VariantWrap(v).parent_refs().and_then(|parent_refs| {
                    let layers: Vec<DiagLayer<'a>> = parent_refs
                        .iter()
                        .filter_map(|pr| get_layer(cda_database::datatypes::ParentRef(pr)))
                        .collect();
                    (!layers.is_empty()).then_some(layers)
                })
            })
            .flatten()
            .collect();
        all.extend(variant_layers);
    }

    // Collect from functional groups
    let fg_layers: Vec<DiagLayer<'a>> = ecu
        .functional_groups()
        .into_iter()
        .flatten()
        .filter_map(|fg| {
            fg.parent_refs().and_then(|parent_refs| {
                let layers: Vec<DiagLayer<'a>> = parent_refs
                    .iter()
                    .filter_map(|pr| get_layer(cda_database::datatypes::ParentRef(pr)))
                    .collect();
                (!layers.is_empty()).then_some(layers)
            })
        })
        .flatten()
        .collect();
    all.extend(fg_layers);

    let mut seen = std::collections::HashSet::new();
    all.into_iter()
        .filter(|dl| {
            let name = dl.short_name().unwrap_or("");
            !name.is_empty() && seen.insert(name.to_owned())
        })
        .collect()
}

/// Add a section of `DiagLayer`s to the tree (section header + one child node
/// per layer with its full sub-structure).
fn add_layer_section(
    b: &mut TreeBuilder,
    layers: &[DiagLayer<'_>],
    section_name: &str,
    section_type: SectionType,
    overview_title: &str,
) {
    if layers.is_empty() {
        return;
    }

    let names: Vec<String> = layers
        .iter()
        .filter_map(|dl| dl.short_name().map(str::to_owned))
        .collect();
    let overview = build_names_overview_table(&names, overview_title);

    b.push_section_header(
        section_name.to_string(),
        false,
        true,
        overview,
        section_type,
    );

    for layer in layers {
        let name = layer.short_name().unwrap_or("unnamed");
        let detail_sections = build_layer_summary_section(layer, name);

        // ECU Shared Data / Protocols are at the top of the hierarchy — no parent refs
        b.push_container(1, name.to_string(), detail_sections, Vec::new());

        b.add_diag_layer_structured(
            layer,
            2,
            None::<std::iter::Empty<cda_database::datatypes::ParentRef>>,
            None::<std::iter::Empty<cda_database::datatypes::Variant>>,
        );
    }
}

/// Add all ECU shared data to the tree
pub fn add_ecu_shared_data(b: &mut TreeBuilder, ecu: &EcuDb<'_>) {
    let layers = collect_parent_ref_layers(ecu, |pr| match pr.ref_type().try_into() {
        Ok(cda_database::datatypes::ParentRefType::EcuSharedData) => pr
            .ref__as_ecu_shared_data()
            .and_then(|esd| esd.diag_layer().map(DiagLayer)),
        Ok(
            cda_database::datatypes::ParentRefType::Variant
            | cda_database::datatypes::ParentRefType::Protocol
            | cda_database::datatypes::ParentRefType::FunctionalGroup
            | cda_database::datatypes::ParentRefType::TableDop
            | cda_database::datatypes::ParentRefType::NONE,
        )
        | Err(_) => None,
    });
    add_layer_section(
        b,
        &layers,
        "ECU Shared Data",
        SectionType::EcuSharedData,
        "ECU Shared Data Overview",
    );
}

/// Add all protocols to the tree
pub fn add_protocols(b: &mut TreeBuilder, ecu: &EcuDb<'_>) {
    let layers = collect_parent_ref_layers(ecu, |pr| match pr.ref_type().try_into() {
        Ok(cda_database::datatypes::ParentRefType::Protocol) => pr
            .ref__as_protocol()
            .and_then(|p| p.diag_layer().map(DiagLayer)),
        Ok(
            cda_database::datatypes::ParentRefType::Variant
            | cda_database::datatypes::ParentRefType::EcuSharedData
            | cda_database::datatypes::ParentRefType::FunctionalGroup
            | cda_database::datatypes::ParentRefType::TableDop
            | cda_database::datatypes::ParentRefType::NONE,
        )
        | Err(_) => None,
    });

    if layers.is_empty() {
        return;
    }

    let names: Vec<String> = layers
        .iter()
        .filter_map(|dl| dl.short_name().map(str::to_owned))
        .collect();
    let overview = build_names_overview_table(&names, "Protocols Overview");

    b.push_section_header(
        "Protocols".to_string(),
        false,
        true,
        overview,
        SectionType::Protocols,
    );

    for layer in &layers {
        let name = layer.short_name().unwrap_or("unnamed");
        let detail_sections = build_layer_summary_section(layer, name);

        // Protocols are at the top of the hierarchy — their parent refs are handled
        // by the layers that reference them (variants, functional groups)
        b.push_container(1, name.to_string(), detail_sections, Vec::new());

        b.add_diag_layer_structured(
            layer,
            2,
            None::<std::iter::Empty<cda_database::datatypes::ParentRef>>,
            None::<std::iter::Empty<cda_database::datatypes::Variant>>,
        );
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

/// Build summary section for a `DiagLayer` (used by functional groups and ECU shared data)
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

/// Append `DiagLayer` info rows (short name, long name, children) to an existing row list
fn append_layer_info_rows(layer: &DiagLayer<'_>, info_rows: &mut Vec<DetailRow>) {
    if let Some(sn) = layer.short_name() {
        info_rows.push(DetailRow::normal(
            vec!["Short Name".to_owned(), sn.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ));
    }
    if let Some(ln) = layer.long_name() {
        let value = ln.value().unwrap_or("-");
        let ti = ln.ti().unwrap_or("-");
        info_rows.push(DetailRow::normal(
            vec!["Long Name".to_owned(), format!("value: {value}, ti: {ti}")],
            vec![CellType::Text, CellType::Text],
            0,
        ));
    }

    if let Some(children_rows) = build_children_rows(layer) {
        info_rows.push(DetailRow::normal(
            vec![String::new(), String::new()],
            vec![CellType::Text, CellType::Text],
            0,
        ));
        info_rows.push(DetailRow::normal(
            vec!["Children".to_owned(), String::new()],
            vec![CellType::Text, CellType::Text],
            0,
        ));
        info_rows.extend(children_rows);
    }
}

/// Build rows listing all child elements with counts
fn build_children_rows(layer: &DiagLayer<'_>) -> Option<Vec<DetailRow>> {
    let service_count = layer.diag_services().map_or(0, |s| s.len());
    let job_count = layer.single_ecu_jobs().map_or(0, |j| j.len());
    let neg_count: usize = layer.diag_services().map_or(0, |services| {
        services
            .iter()
            .filter_map(|s| DiagService(s).neg_responses().map(|r| r.len()))
            .sum()
    });
    let pos_count: usize = layer.diag_services().map_or(0, |services| {
        services
            .iter()
            .filter_map(|s| DiagService(s).pos_responses().map(|r| r.len()))
            .sum()
    });
    let request_count = layer.diag_services().map_or(0, |services| {
        services
            .iter()
            .filter(|&s| DiagService(s).request().is_some())
            .count()
    });

    let rows: Vec<DetailRow> = [
        build_child_row(
            "ComParam Refs",
            layer
                .com_param_refs()
                .filter(|r| !r.is_empty())
                .map(|r| r.len().to_string()),
            ChildElementType::ComParamRefs,
        ),
        build_child_row(
            "Diag-Comms",
            (service_count.saturating_add(job_count) > 0)
                .then(|| format!("{service_count} services, {job_count} jobs")),
            ChildElementType::DiagComms,
        ),
        build_child_row(
            "Functional Classes",
            layer
                .funct_classes()
                .filter(|f| !f.is_empty())
                .map(|f| f.len().to_string()),
            ChildElementType::FunctionalClasses,
        ),
        build_child_row(
            "Neg-Responses",
            (neg_count > 0).then(|| neg_count.to_string()),
            ChildElementType::NegResponses,
        ),
        build_child_row(
            "Pos-Responses",
            (pos_count > 0).then(|| pos_count.to_string()),
            ChildElementType::PosResponses,
        ),
        build_child_row(
            "Requests",
            (request_count > 0).then(|| request_count.to_string()),
            ChildElementType::Requests,
        ),
        build_child_row(
            "SDGs",
            layer
                .sdgs()
                .and_then(|s| s.sdgs())
                .filter(|l| !l.is_empty())
                .map(|l| l.len().to_string()),
            ChildElementType::SDGs,
        ),
        build_child_row(
            "State Charts",
            layer
                .state_charts()
                .filter(|c| !c.is_empty())
                .map(|c| c.len().to_string()),
            ChildElementType::StateCharts,
        ),
    ]
    .into_iter()
    .flatten()
    .collect();

    (!rows.is_empty()).then_some(rows)
}

/// Build a single child-element row. Returns `None` when `value` is `None`
/// (i.e., the element is absent or empty).
fn build_child_row(
    label: &str,
    value: Option<String>,
    element_type: ChildElementType,
) -> Option<DetailRow> {
    let value = value?;
    Some(DetailRow {
        cells: vec![label.to_owned(), value],
        cell_types: vec![CellType::ParameterName, CellType::Text],
        cell_jump_targets: vec![None; 2],
        indent: 0,
        row_type: DetailRowType::ChildElement,
        metadata: Some(RowMetadata::ChildElement { element_type }),
    })
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

    let rows: Vec<_> = variants
        .iter()
        .map(|variant| {
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

            DetailRow::with_jump_targets(
                vec![name, is_base.to_owned()],
                vec![CellType::ParameterName, CellType::Text],
                vec![Some(CellJumpTarget::ContainerByName), None],
                0,
            )
        })
        .collect();

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

/// Build a simple overview table with a "Short Name" column for a list of names.
/// Used by Functional Groups, ECU Shared Data, and Protocols section headers.
fn build_names_overview_table(names: &[String], title: &str) -> Vec<DetailSectionData> {
    if names.is_empty() {
        return vec![];
    }

    let header = DetailRow::header(vec!["Short Name".to_owned()], vec![CellType::Text]);

    let rows: Vec<DetailRow> = names
        .iter()
        .map(|name| DetailRow {
            cells: vec![name.clone()],
            cell_types: vec![CellType::ParameterName],
            cell_jump_targets: vec![Some(CellJumpTarget::TreeNodeByName)],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        })
        .collect();

    vec![
        DetailSectionData::new(
            title.to_owned(),
            DetailContent::Table {
                header,
                rows,
                constraints: vec![ColumnConstraint::Percentage(100)],
                use_row_selection: true,
            },
            false,
        )
        .with_type(DetailSectionType::Overview),
    ]
}
