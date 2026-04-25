/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::DiagLayer;

use super::dops::parse_dop_name;
use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellJumpTarget, CellJumpTargetType, CellType, ColumnConstraint, DetailCell, DetailContent,
        DetailRow, DetailSectionData, DetailSectionType, NodeType,
    },
};

/// Map a `param_class` string to a short display badge prefix, e.g. `"[TIMING] "`.
/// Returns an empty `String` when `param_class` is absent or empty.
fn param_class_badge(param_class: Option<&str>) -> String {
    match param_class {
        Some(cls) if !cls.is_empty() && cls != "-" => format!("[{cls}] "),
        _ => String::new(),
    }
}

/// Format a value as hex with decimal in parentheses if it's numeric.
/// E.g. "255" -> "0xFF (255)", "abc" -> "abc"
fn format_value_hex_decimal(value: &str) -> String {
    value
        .parse::<i64>()
        .map_or_else(|_| value.to_owned(), |n| format!("0x{n:X} ({n})"))
}

/// Add `ComParam` refs section to the tree
pub fn add_com_params(b: &mut TreeBuilder, layer: &DiagLayer<'_>, depth: usize) {
    let Some(cp_refs) = layer.com_param_refs() else {
        return;
    };
    if cp_refs.is_empty() {
        return;
    }

    // Push header first (empty details — patched below with indices).
    let header_idx = b.next_index();
    b.push_service_list_header(
        depth,
        format!("ComParam Refs ({})", cp_refs.len()),
        false,
        true,
        vec![],
        crate::tree::ServiceListType::ComParamRefs,
    );

    // Collect and sort by name
    let mut sorted_refs: Vec<_> = cp_refs
        .iter()
        .enumerate()
        .filter_map(|(idx, cpr)| {
            let cp = cpr.com_param()?;
            let name = cp.short_name().unwrap_or("?").to_owned();
            let param_class = cp.param_class().map(std::borrow::ToOwned::to_owned);
            Some((idx, name, param_class))
        })
        .collect();
    sorted_refs.sort_by(|a, b| a.1.cmp(&b.1));

    // Push children, keying node_indices by the badged display name so that
    // resolve_all_indices can match jump targets against node.text after re-sorts.
    let mut node_indices = std::collections::HashMap::new();
    for (idx, cp_name, param_class) in &sorted_refs {
        let badge = param_class_badge(param_class.as_deref());
        let display_name = format!("{badge}{cp_name}");
        node_indices.insert(display_name.clone(), b.next_index());
        let sections = build_com_param_ref_detail(layer, *idx);
        b.push_details_structured(
            depth.saturating_add(1),
            display_name,
            false,
            false,
            sections,
            NodeType::Default,
        );
    }

    // Build overview with tree-node indices and patch header.
    let overview = build_com_params_overview(layer, &node_indices);
    b.set_detail_sections(header_idx, overview);
}

fn build_com_params_overview(
    layer: &DiagLayer<'_>,
    node_indices: &std::collections::HashMap<String, usize>,
) -> Vec<DetailSectionData> {
    let Some(cp_refs) = layer.com_param_refs() else {
        return vec![];
    };

    let header = DetailRow::header(vec![
        DetailCell::text("Short Name"),
        DetailCell::text("Type"),
    ]);

    let mut rows: Vec<DetailRow> = cp_refs
        .iter()
        .filter_map(|cpr| {
            let cp = cpr.com_param()?;
            let name = cp.short_name().unwrap_or("?").to_owned();
            let cp_type = format!("{:?}", cp.com_param_type());
            let badge = param_class_badge(cp.param_class());
            let display_name = format!("{badge}{name}");
            let jump = make_index_jump(&display_name, node_indices);
            Some(DetailRow::normal(
                vec![
                    DetailCell::new(name, CellType::ParameterName).with_jump(jump),
                    DetailCell::text(cp_type),
                ],
                0,
            ))
        })
        .collect();
    rows.sort_by(|a, b| {
        let a_name = a.cell_text(0);
        let b_name = b.cell_text(0);
        a_name.cmp(b_name)
    });

    vec![
        DetailSectionData::new(
            "Overview".to_owned(),
            DetailContent::Table {
                header,
                rows,
                constraints: vec![
                    ColumnConstraint::Percentage(60),
                    ColumnConstraint::Percentage(40),
                ],
                use_row_selection: true,
            },
            false,
        )
        .with_type(DetailSectionType::Overview),
    ]
}

fn make_index_jump(
    short_name: &str,
    node_indices: &std::collections::HashMap<String, usize>,
) -> CellJumpTarget {
    let index = node_indices.get(short_name).copied().unwrap_or(usize::MAX);
    CellJumpTarget::new(CellJumpTargetType::TreeNodeByIndex {
        index,
        short_name: short_name.to_owned(),
    })
}

/// Helper to create a simple key-value row without jump target.
fn kv_row(key: &str, value: String) -> DetailRow {
    DetailRow::normal(vec![DetailCell::text(key), DetailCell::text(value)], 0)
}

/// Helper to create a key-value row with a DOP reference (jump cell).
/// Uses parsed name parts to show a more readable display name.
fn dop_row(key: &str, dop_name: &str) -> DetailRow {
    let parsed = parse_dop_name(dop_name);
    let display = parsed.display_name();
    let value = if display.is_empty() {
        dop_name.to_owned()
    } else {
        display.clone()
    };
    // Use the display name for navigation target since tree nodes also use display names
    let nav_name = if display.is_empty() {
        dop_name.to_owned()
    } else {
        display
    };
    DetailRow::normal(
        vec![
            DetailCell::text(key),
            DetailCell::new(value, CellType::DopReference).with_jump(CellJumpTarget::new(
                CellJumpTargetType::Dop {
                    index: usize::MAX,
                    name: nav_name,
                },
            )),
        ],
        0,
    )
}

fn build_general_section(layer: &DiagLayer<'_>, idx: usize) -> Option<DetailSectionData> {
    let cp_refs = layer.com_param_refs()?;
    if idx >= cp_refs.len() {
        return None;
    }
    let cpr = cp_refs.get(idx);
    let mut rows: Vec<DetailRow> = Vec::new();

    if let Some(cp) = cpr.com_param() {
        rows.push(kv_row(
            "Short Name",
            cp.short_name().unwrap_or("?").to_owned(),
        ));
        let com_param_type_str = format!("{:?}", cp.com_param_type());
        rows.push(kv_row("Type", com_param_type_str.clone()));

        // Show the actual specific data type from the union
        let specific_data_type_raw = format!("{:?}", cp.specific_data_type());
        let specific_data_type = specific_data_type_raw.trim_matches('"');

        // Detect mismatch between com_param_type enum and actual specific_data union
        let has_regular_data = cp.specific_data_as_regular_com_param().is_some();
        let has_complex_data = cp.specific_data_as_complex_com_param().is_some();
        let is_type_regular = com_param_type_str == "REGULAR";
        let is_type_complex = com_param_type_str == "COMPLEX";

        let mismatch = (is_type_regular && !has_regular_data)
            || (is_type_complex && !has_complex_data)
            || (!has_regular_data && !has_complex_data);

        let specific_data_display = if mismatch {
            format!("{specific_data_type} (MISMATCH: Type={com_param_type_str})")
        } else {
            specific_data_type.to_owned()
        };
        rows.push(kv_row("Specific Data Type", specific_data_display));

        rows.push(kv_row(
            "Param Class",
            cp.param_class().unwrap_or("-").to_owned(),
        ));
        rows.push(kv_row(
            "Standardisation Level",
            format!("{:?}", cp.cp_type()),
        ));
        rows.push(kv_row("Usage", format!("{:?}", cp.cp_usage())));

        if let Some(dl) = cp.display_level() {
            rows.push(kv_row("Display Level", dl.to_string()));
        }

        if let Some(rcp) = cp.specific_data_as_regular_com_param() {
            if let Some(val) = rcp.physical_default_value() {
                rows.push(kv_row(
                    "Physical Default Value",
                    format_value_hex_decimal(val),
                ));
            }
            // Add the associated DOP with jump target
            if let Some(dop) = rcp.dop() {
                let dop_name = dop.short_name().unwrap_or("?");
                rows.push(dop_row("DOP", dop_name));
            }
        }
    }

    if let Some(sv) = cpr.simple_value()
        && let Some(val) = sv.value()
    {
        rows.push(kv_row("Simple Value", format_value_hex_decimal(val)));
    }

    if let Some(proto) = cpr.protocol()
        && let Some(dl) = proto.diag_layer()
        && let Some(name) = dl.short_name()
    {
        rows.push(kv_row("Protocol", name.to_owned()));
    }

    if let Some(ps) = cpr.prot_stack()
        && let Some(name) = ps.short_name()
    {
        rows.push(kv_row("Prot Stack", name.to_owned()));
    }

    if rows.is_empty() {
        return None;
    }

    let header = DetailRow::header(vec![
        DetailCell::text("Property"),
        DetailCell::text("Value"),
    ]);

    Some(
        DetailSectionData::new(
            "General".to_owned(),
            DetailContent::Table {
                header,
                rows,
                constraints: vec![
                    ColumnConstraint::Percentage(40),
                    ColumnConstraint::Percentage(60),
                ],
                use_row_selection: false,
            },
            false,
        )
        .with_type(DetailSectionType::Overview),
    )
}

fn build_complex_value_section(layer: &DiagLayer<'_>, idx: usize) -> Option<DetailSectionData> {
    let cp_refs = layer.com_param_refs()?;
    if idx >= cp_refs.len() {
        return None;
    }
    let cpr = cp_refs.get(idx);
    let cv = cpr.complex_value()?;
    let entries_type = cv.entries_type()?;

    let cv_rows: Vec<DetailRow> = entries_type
        .iter()
        .enumerate()
        .map(|(i, tag)| {
            let value = cv
                .entries_item_as_simple_value(i)
                .and_then(|sv| sv.value().map(format_value_hex_decimal))
                .unwrap_or_else(|| format!("Complex[{i}]"));
            DetailRow::normal(
                vec![
                    DetailCell::text(format!("{i}")),
                    DetailCell::text(format!("{tag:?}")),
                    DetailCell::text(value),
                ],
                0,
            )
        })
        .collect();

    if cv_rows.is_empty() {
        return None;
    }

    let header = DetailRow::header(vec![
        DetailCell::text("#"),
        DetailCell::text("Type"),
        DetailCell::text("Value"),
    ]);
    Some(
        DetailSectionData::new(
            "Complex Value".to_owned(),
            DetailContent::Table {
                header,
                rows: cv_rows,
                constraints: vec![
                    ColumnConstraint::Fixed(5),
                    ColumnConstraint::Percentage(30),
                    ColumnConstraint::Percentage(70),
                ],
                use_row_selection: false,
            },
            false,
        )
        .with_type(DetailSectionType::Custom),
    )
}

fn build_sub_params_section(layer: &DiagLayer<'_>, idx: usize) -> Option<DetailSectionData> {
    let cp_refs = layer.com_param_refs()?;
    if idx >= cp_refs.len() {
        return None;
    }
    let cpr = cp_refs.get(idx);
    let cp = cpr.com_param()?;
    let ccp = cp.specific_data_as_complex_com_param()?;
    let sub_params = ccp.com_params()?;

    let header = DetailRow::header(vec![
        DetailCell::text("Short Name"),
        DetailCell::text("Type"),
        DetailCell::text("Specific Data"),
        DetailCell::text("Param Class"),
        DetailCell::text("Default Value"),
    ]);
    let rows: Vec<DetailRow> = sub_params
        .iter()
        .map(|sp| {
            let name = sp.short_name().unwrap_or("?").to_owned();
            let sp_type = format!("{:?}", sp.com_param_type());
            let param_class = sp.param_class().unwrap_or("-").to_owned();
            let default_val = sp
                .specific_data_as_regular_com_param()
                .and_then(|r| r.physical_default_value().map(format_value_hex_decimal))
                .unwrap_or_default();

            // Show specific data type with mismatch detection
            let specific_data_raw = format!("{:?}", sp.specific_data_type());
            let specific_data_type = specific_data_raw.trim_matches('"');
            let has_regular = sp.specific_data_as_regular_com_param().is_some();
            let has_complex = sp.specific_data_as_complex_com_param().is_some();
            let is_regular = sp_type == "REGULAR";
            let is_complex = sp_type == "COMPLEX";
            let mismatch = (is_regular && !has_regular)
                || (is_complex && !has_complex)
                || (!has_regular && !has_complex);
            let specific_data_display = if mismatch {
                format!("{specific_data_type} (MISMATCH)")
            } else {
                specific_data_type.to_owned()
            };

            DetailRow::normal(
                vec![
                    DetailCell::text(name),
                    DetailCell::text(sp_type),
                    DetailCell::text(specific_data_display),
                    DetailCell::text(param_class),
                    DetailCell::text(default_val),
                ],
                0,
            )
        })
        .collect();

    if rows.is_empty() {
        return None;
    }

    Some(
        DetailSectionData::new(
            "Sub-Parameters".to_owned(),
            DetailContent::Table {
                header,
                rows,
                constraints: vec![
                    ColumnConstraint::Percentage(25),
                    ColumnConstraint::Percentage(15),
                    ColumnConstraint::Percentage(20),
                    ColumnConstraint::Percentage(15),
                    ColumnConstraint::Percentage(25),
                ],
                use_row_selection: true,
            },
            false,
        )
        .with_type(DetailSectionType::ComParams),
    )
}

fn build_com_param_ref_detail(layer: &DiagLayer<'_>, idx: usize) -> Vec<DetailSectionData> {
    [
        build_general_section(layer, idx),
        build_complex_value_section(layer, idx),
        build_sub_params_section(layer, idx),
    ]
    .into_iter()
    .flatten()
    .collect()
}
