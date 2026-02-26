// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use cda_database::datatypes::ParentRef;

use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ColumnConstraint, DetailContent, DetailRow, DetailSectionData, DetailSectionType,
        NodeType,
    },
};

/// Add a Parent Refs section with an overview table at the section level,
/// and individual parent refs as children in the tree with their own detail views.
pub fn add_parent_refs_with_details<'a>(
    b: &mut TreeBuilder,
    depth: usize,
    parent_refs: Option<impl Iterator<Item = ParentRef<'a>>>,
) {
    let Some(parent_refs_iter) = parent_refs else {
        return;
    };

    let parent_refs_list: Vec<_> = parent_refs_iter.collect();

    if parent_refs_list.is_empty() {
        return;
    }

    let overview = build_parent_refs_overview(&parent_refs_list);

    b.push_details_structured(
        depth,
        format!("Parent Refs ({})", parent_refs_list.len()),
        false,
        true,
        vec![overview],
        NodeType::ParentRefs,
    );

    for parent_ref in &parent_refs_list {
        let (ref_type_str, short_name) = extract_parent_ref_info(parent_ref);
        let detail_sections = build_single_parent_ref_detail(parent_ref, &short_name, ref_type_str);

        b.push_details_structured(
            depth.saturating_add(1),
            short_name,
            false,
            false,
            detail_sections,
            NodeType::Default,
        );
    }
}

fn extract_parent_ref_info(parent_ref: &ParentRef<'_>) -> (&'static str, String) {
    match parent_ref.ref_type().try_into() {
        Ok(cda_database::datatypes::ParentRefType::Variant) => {
            let short_name = parent_ref
                .ref__as_variant()
                .and_then(|v| v.diag_layer())
                .and_then(|dl| dl.short_name())
                .unwrap_or("?")
                .to_owned();
            ("Variant", short_name)
        }
        Ok(cda_database::datatypes::ParentRefType::EcuSharedData) => {
            let short_name = parent_ref
                .ref__as_ecu_shared_data()
                .and_then(|esd| esd.diag_layer())
                .and_then(|dl| dl.short_name())
                .unwrap_or("?")
                .to_owned();
            ("ECU Shared Data", short_name)
        }
        Ok(cda_database::datatypes::ParentRefType::Protocol) => {
            let short_name = parent_ref
                .ref__as_protocol()
                .and_then(|p| p.diag_layer())
                .and_then(|dl| dl.short_name())
                .unwrap_or("?")
                .to_owned();
            ("Protocol", short_name)
        }
        Ok(cda_database::datatypes::ParentRefType::FunctionalGroup) => {
            let short_name = parent_ref
                .ref__as_functional_group()
                .and_then(|fg| fg.diag_layer())
                .and_then(|dl| dl.short_name())
                .unwrap_or("?")
                .to_owned();
            ("Functional Group", short_name)
        }
        _ => ("Unknown", "?".to_owned()),
    }
}

fn build_parent_refs_overview(parent_refs_list: &[ParentRef<'_>]) -> DetailSectionData {
    let header = DetailRow::header(
        vec!["Short Name".to_owned(), "Type".to_owned()],
        vec![CellType::Text, CellType::Text],
    );

    let rows: Vec<DetailRow> = parent_refs_list
        .iter()
        .map(|pr| {
            let (ref_type, name) = extract_parent_ref_info(pr);
            DetailRow::with_jump_targets(
                vec![name, ref_type.to_owned()],
                vec![CellType::ParameterName, CellType::Text],
                vec![Some(crate::tree::CellJumpTarget::ContainerByName), None],
                0,
            )
        })
        .collect();

    DetailSectionData::new(
        "Overview".to_owned(),
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
    .with_type(DetailSectionType::Overview)
}

fn build_single_parent_ref_detail(
    parent_ref: &ParentRef<'_>,
    short_name: &str,
    ref_type: &str,
) -> Vec<DetailSectionData> {
    let mut sections = Vec::new();

    // General info
    let general_header = DetailRow::header(
        vec!["Property".to_owned(), "Value".to_owned()],
        vec![CellType::Text, CellType::Text],
    );
    let general_rows = vec![
        DetailRow::normal(
            vec!["Short Name".to_owned(), short_name.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
        DetailRow::normal(
            vec!["Type".to_owned(), ref_type.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
    ];
    sections.push(
        DetailSectionData::new(
            "General".to_owned(),
            DetailContent::Table {
                header: general_header,
                rows: general_rows,
                constraints: vec![
                    ColumnConstraint::Percentage(40),
                    ColumnConstraint::Percentage(60),
                ],
                use_row_selection: false,
            },
            false,
        )
        .with_type(DetailSectionType::Overview),
    );

    // Not-inherited DiagComms
    if let Some(names) = parent_ref.not_inherited_diag_comm_short_names() {
        let rows: Vec<DetailRow> = names
            .iter()
            .map(|name| DetailRow::normal(vec![name.to_owned()], vec![CellType::Text], 0))
            .collect();
        if !rows.is_empty() {
            sections.push(build_not_inherited_section("Not Inherited DiagComms", rows));
        }
    }

    // Not-inherited DiagVariables
    if let Some(names) = parent_ref.not_inherited_variables_short_names() {
        let rows: Vec<DetailRow> = names
            .iter()
            .map(|name| DetailRow::normal(vec![name.to_owned()], vec![CellType::Text], 0))
            .collect();
        if !rows.is_empty() {
            sections.push(build_not_inherited_section("Not Inherited Variables", rows));
        }
    }

    // Not-inherited DOPs
    if let Some(names) = parent_ref.not_inherited_dops_short_names() {
        let rows: Vec<DetailRow> = names
            .iter()
            .map(|name| DetailRow::normal(vec![name.to_owned()], vec![CellType::Text], 0))
            .collect();
        if !rows.is_empty() {
            sections.push(build_not_inherited_section("Not Inherited DOPs", rows));
        }
    }

    // Not-inherited Tables
    if let Some(names) = parent_ref.not_inherited_tables_short_names() {
        let rows: Vec<DetailRow> = names
            .iter()
            .map(|name| DetailRow::normal(vec![name.to_owned()], vec![CellType::Text], 0))
            .collect();
        if !rows.is_empty() {
            sections.push(build_not_inherited_section("Not Inherited Tables", rows));
        }
    }

    sections
}

fn build_not_inherited_section(title: &str, rows: Vec<DetailRow>) -> DetailSectionData {
    let header = DetailRow::header(vec!["Short Name".to_owned()], vec![CellType::Text]);

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
    .with_type(DetailSectionType::Custom)
}
