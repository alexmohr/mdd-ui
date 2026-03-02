/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::ParentRef;

use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellJumpTarget, CellType, ColumnConstraint, DetailContent, DetailRow, DetailSectionData,
        DetailSectionType, NodeType,
    },
};

/// Extract the short names of all containers referenced by the given parent refs.
/// Used at build time to store the DB hierarchy on `TreeNode` for navigation.
pub fn extract_parent_ref_short_names<'a>(
    parent_refs: Option<impl Iterator<Item = ParentRef<'a>>>,
) -> Vec<String> {
    let Some(refs) = parent_refs else {
        return Vec::new();
    };
    refs.filter_map(|pr| {
        let (_, name) = extract_parent_ref_info(&pr);
        (name != "?").then_some(name)
    })
    .collect()
}

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
        Ok(cda_database::datatypes::ParentRefType::TableDop) => ("Table DOP", "?".to_owned()),
        Ok(cda_database::datatypes::ParentRefType::NONE) | Err(_) => ("Unknown", "?".to_owned()),
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

    // Not-inherited sections driven by a config table
    let not_inherited_configs = [
        (
            "Not Inherited DiagComms",
            parent_ref.not_inherited_diag_comm_short_names(),
            DetailSectionType::NotInheritedDiagComms,
            make_tree_node_row as fn(&str) -> DetailRow,
        ),
        (
            "Not Inherited Variables",
            parent_ref.not_inherited_variables_short_names(),
            DetailSectionType::NotInheritedVariables,
            make_tree_node_row as fn(&str) -> DetailRow,
        ),
        (
            "Not Inherited DOPs",
            parent_ref.not_inherited_dops_short_names(),
            DetailSectionType::NotInheritedDops,
            make_dop_row as fn(&str) -> DetailRow,
        ),
        (
            "Not Inherited Tables",
            parent_ref.not_inherited_tables_short_names(),
            DetailSectionType::NotInheritedTables,
            make_tree_node_row as fn(&str) -> DetailRow,
        ),
    ];
    for (title, names_opt, section_type, make_row) in not_inherited_configs {
        let Some(names) = names_opt else { continue };
        let rows: Vec<DetailRow> = names.iter().map(make_row).collect();
        if !rows.is_empty() {
            sections.push(build_not_inherited_section(title, rows, section_type));
        }
    }

    sections
}

fn build_not_inherited_section(
    title: &str,
    rows: Vec<DetailRow>,
    section_type: DetailSectionType,
) -> DetailSectionData {
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
    .with_type(section_type)
}

fn make_tree_node_row(name: &str) -> DetailRow {
    DetailRow::with_jump_targets(
        vec![name.to_owned()],
        vec![CellType::ParameterName],
        vec![Some(CellJumpTarget::TreeNodeByName)],
        0,
    )
}

fn make_dop_row(name: &str) -> DetailRow {
    let dop_name = name.to_owned();
    DetailRow::with_jump_targets(
        vec![dop_name.clone()],
        vec![CellType::DopReference],
        vec![Some(CellJumpTarget::Dop { name: dop_name })],
        0,
    )
}
