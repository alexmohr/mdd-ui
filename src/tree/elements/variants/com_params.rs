/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::DiagLayer;

use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellJumpTarget, CellType, ColumnConstraint, DetailContent, DetailRow, DetailSectionData,
        DetailSectionType, NodeType,
    },
};

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

    let overview = build_com_params_overview(layer);

    b.push_details_structured(
        depth,
        format!("ComParam Refs ({})", cp_refs.len()),
        false,
        true,
        overview,
        NodeType::SectionHeader,
    );

    // Collect and sort by name
    let mut sorted_refs: Vec<_> = cp_refs
        .iter()
        .enumerate()
        .filter_map(|(idx, cpr)| {
            let cp = cpr.com_param()?;
            let name = cp.short_name().unwrap_or("?").to_owned();
            Some((idx, name))
        })
        .collect();
    sorted_refs.sort_by(|a, b| a.1.cmp(&b.1));

    for (idx, cp_name) in sorted_refs {
        let sections = build_com_param_ref_detail(layer, idx);
        b.push_details_structured(
            depth.saturating_add(1),
            cp_name,
            false,
            false,
            sections,
            NodeType::Default,
        );
    }
}

fn build_com_params_overview(layer: &DiagLayer<'_>) -> Vec<DetailSectionData> {
    let Some(cp_refs) = layer.com_param_refs() else {
        return vec![];
    };

    let header = DetailRow::header(
        vec!["Short Name".to_owned(), "Type".to_owned()],
        vec![CellType::Text, CellType::Text],
    );

    let mut rows: Vec<DetailRow> = cp_refs
        .iter()
        .filter_map(|cpr| {
            let cp = cpr.com_param()?;
            let name = cp.short_name().unwrap_or("?").to_owned();
            let cp_type = format!("{:?}", cp.com_param_type());
            Some(DetailRow::with_jump_targets(
                vec![name, cp_type],
                vec![CellType::ParameterName, CellType::Text],
                vec![Some(CellJumpTarget::TreeNodeByName), None],
                0,
            ))
        })
        .collect();
    rows.sort_by(|a, b| {
        let a_name = a.cells.first().map_or("", String::as_str);
        let b_name = b.cells.first().map_or("", String::as_str);
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

fn build_general_section(layer: &DiagLayer<'_>, idx: usize) -> Option<DetailSectionData> {
    let cp_refs = layer.com_param_refs()?;
    if idx >= cp_refs.len() {
        return None;
    }
    let cpr = cp_refs.get(idx);
    let mut general_rows: Vec<(String, String)> = Vec::new();

    if let Some(cp) = cpr.com_param() {
        general_rows.push((
            "Short Name".to_owned(),
            cp.short_name().unwrap_or("?").to_owned(),
        ));
        let com_param_type_str = format!("{:?}", cp.com_param_type());
        general_rows.push(("Type".to_owned(), com_param_type_str.clone()));

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
        general_rows.push(("Specific Data Type".to_owned(), specific_data_display));

        general_rows.push((
            "Param Class".to_owned(),
            cp.param_class().unwrap_or("-").to_owned(),
        ));
        general_rows.push((
            "Standardisation Level".to_owned(),
            format!("{:?}", cp.cp_type()),
        ));
        general_rows.push(("Usage".to_owned(), format!("{:?}", cp.cp_usage())));

        if let Some(dl) = cp.display_level() {
            general_rows.push(("Display Level".to_owned(), dl.to_string()));
        }

        if let Some(rcp) = cp.specific_data_as_regular_com_param()
            && let Some(val) = rcp.physical_default_value()
        {
            general_rows.push((
                "Physical Default Value".to_owned(),
                format_value_hex_decimal(val),
            ));
        }
    }

    if let Some(sv) = cpr.simple_value()
        && let Some(val) = sv.value()
    {
        general_rows.push(("Simple Value".to_owned(), format_value_hex_decimal(val)));
    }

    if let Some(proto) = cpr.protocol()
        && let Some(dl) = proto.diag_layer()
        && let Some(name) = dl.short_name()
    {
        general_rows.push(("Protocol".to_owned(), name.to_owned()));
    }

    if let Some(ps) = cpr.prot_stack()
        && let Some(name) = ps.short_name()
    {
        general_rows.push(("Prot Stack".to_owned(), name.to_owned()));
    }

    if general_rows.is_empty() {
        return None;
    }

    let header = DetailRow::header(
        vec!["Property".to_owned(), "Value".to_owned()],
        vec![CellType::Text, CellType::Text],
    );
    let rows: Vec<DetailRow> = general_rows
        .into_iter()
        .map(|(k, v)| DetailRow::normal(vec![k, v], vec![CellType::Text, CellType::Text], 0))
        .collect();

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
                vec![format!("{i}"), format!("{tag:?}"), value],
                vec![CellType::Text, CellType::Text, CellType::Text],
                0,
            )
        })
        .collect();

    if cv_rows.is_empty() {
        return None;
    }

    let header = DetailRow::header(
        vec!["#".to_owned(), "Type".to_owned(), "Value".to_owned()],
        vec![CellType::Text, CellType::Text, CellType::Text],
    );
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

    let header = DetailRow::header(
        vec![
            "Short Name".to_owned(),
            "Type".to_owned(),
            "Specific Data".to_owned(),
            "Param Class".to_owned(),
            "Default Value".to_owned(),
        ],
        vec![
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
        ],
    );
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
                    name,
                    sp_type,
                    specific_data_display,
                    param_class,
                    default_val,
                ],
                vec![
                    CellType::Text,
                    CellType::Text,
                    CellType::Text,
                    CellType::Text,
                    CellType::Text,
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
