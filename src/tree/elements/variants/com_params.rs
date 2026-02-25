// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use cda_database::datatypes::DiagLayer;

use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ColumnConstraint, DetailContent, DetailRow, DetailSectionData, DetailSectionType,
        NodeType,
    },
};

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

    for (idx, cpr) in cp_refs.iter().enumerate() {
        let Some(cp) = cpr.com_param() else { continue };
        let cp_name = cp.short_name().unwrap_or("?");
        let sections = build_com_param_ref_detail(layer, idx);
        b.push_details_structured(
            depth.saturating_add(1),
            cp_name.to_owned(),
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

    let rows: Vec<DetailRow> = cp_refs
        .iter()
        .filter_map(|cpr| {
            let cp = cpr.com_param()?;
            let name = cp.short_name().unwrap_or("?").to_owned();
            let cp_type = format!("{:?}", cp.com_param_type());
            Some(DetailRow::normal(
                vec![name, cp_type],
                vec![CellType::Text, CellType::Text],
                0,
            ))
        })
        .collect();

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
    let cpr = cp_refs.get(idx);
    let mut general_rows: Vec<(String, String)> = Vec::new();

    if let Some(cp) = cpr.com_param() {
        general_rows.push((
            "Short Name".to_owned(),
            cp.short_name().unwrap_or("?").to_owned(),
        ));
        general_rows.push(("Type".to_owned(), format!("{:?}", cp.com_param_type())));
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
            general_rows.push(("Physical Default Value".to_owned(), val.to_owned()));
        }
    }

    if let Some(sv) = cpr.simple_value()
        && let Some(val) = sv.value()
    {
        general_rows.push(("Simple Value".to_owned(), val.to_owned()));
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
    let cpr = cp_refs.get(idx);
    let cv = cpr.complex_value()?;
    let entries_type = cv.entries_type()?;

    let cv_rows: Vec<DetailRow> = entries_type
        .iter()
        .enumerate()
        .map(|(i, tag)| {
            let value = cv
                .entries_item_as_simple_value(i)
                .and_then(|sv| sv.value().map(std::borrow::ToOwned::to_owned))
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
    let cpr = cp_refs.get(idx);
    let cp = cpr.com_param()?;
    let ccp = cp.specific_data_as_complex_com_param()?;
    let sub_params = ccp.com_params()?;

    let header = DetailRow::header(
        vec![
            "Short Name".to_owned(),
            "Type".to_owned(),
            "Param Class".to_owned(),
            "Default Value".to_owned(),
        ],
        vec![
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
                .and_then(|r| {
                    r.physical_default_value()
                        .map(std::borrow::ToOwned::to_owned)
                })
                .unwrap_or_default();
            DetailRow::normal(
                vec![name, sp_type, param_class, default_val],
                vec![
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
                    ColumnConstraint::Percentage(30),
                    ColumnConstraint::Percentage(20),
                    ColumnConstraint::Percentage(20),
                    ColumnConstraint::Percentage(30),
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
