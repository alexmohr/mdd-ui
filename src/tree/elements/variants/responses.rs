/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::{DiagLayer, DiagService, Parameter, ParentRef};

use super::{
    format_service_display_name, format_service_id,
    services::{extract_coded_value, extract_dop_name, get_parent_ref_services_recursive},
};
use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ColumnConstraint, DetailContent, DetailRow, DetailSectionData, DetailSectionType,
        NodeType,
    },
};

/// Build Pos-Response sections - this is the core rendering logic for Pos-Response data
/// `DiagComm` module should import and use this function to render the Pos-Response tabs
pub fn build_pos_responses_sections(ds: &DiagService<'_>) -> Vec<DetailSectionData> {
    ds.pos_responses()
        .into_iter()
        .flat_map(|pos| pos.iter().enumerate())
        .map(|(i, resp)| {
            let params = resp.params().into_iter().flatten().map(Parameter);
            build_response_param_section(
                &format!("Pos-Response {}", i.saturating_add(1)),
                params,
                DetailSectionType::PosResponses,
            )
        })
        .collect()
}

/// Build Neg-Response sections - this is the core rendering logic for Neg-Response data
/// `DiagComm` module should import and use this function to render the Neg-Response tabs
pub fn build_neg_responses_sections(ds: &DiagService<'_>) -> Vec<DetailSectionData> {
    ds.neg_responses()
        .into_iter()
        .flat_map(|neg| neg.iter().enumerate())
        .map(|(i, resp)| {
            let params = resp.params().into_iter().flatten().map(Parameter);
            build_response_param_section(
                &format!("Neg-Response {}", i.saturating_add(1)),
                params,
                DetailSectionType::NegResponses,
            )
        })
        .collect()
}

/// Add a single service with positive responses to the tree
fn add_pos_response_service(
    b: &mut TreeBuilder,
    ds: &DiagService<'_>,
    depth: usize,
    source_layer: Option<String>,
) {
    let Some(display_name) = format_service_display_name(ds) else {
        return;
    };

    let sections = build_pos_response_view_sections(ds, source_layer);
    let has_params = ds.pos_responses().is_some_and(|r| {
        r.iter()
            .any(|resp| resp.params().is_some_and(|p| !p.is_empty()))
    });

    b.push_details_structured(
        depth.saturating_add(1),
        display_name,
        false,
        has_params,
        sections,
        NodeType::PosResponse,
    );

    let response_count = ds.pos_responses().map_or(0, |r| r.len());
    for (resp_idx, resp) in ds
        .pos_responses()
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
                param_name,
                param_detail,
                NodeType::Default,
                param.id(),
            );
        }
    }
}

/// Add positive responses section to the tree
pub fn add_pos_responses_section<'a>(
    b: &mut TreeBuilder,
    layer: &DiagLayer<'a>,
    depth: usize,
    variant_parent_refs: Option<impl Iterator<Item = ParentRef<'a>> + 'a>,
) {
    let own_services: Vec<DiagService<'_>> = layer
        .diag_services()
        .map(|services| {
            services
                .iter()
                .map(DiagService)
                .filter(|ds| ds.pos_responses().is_some_and(|r| !r.is_empty()))
                .collect()
        })
        .unwrap_or_default();

    let parent_services: Vec<(DiagService<'_>, String)> =
        if let Some(parent_refs) = variant_parent_refs {
            get_parent_ref_services_recursive(parent_refs)
                .into_iter()
                .filter(|(ds, _)| ds.pos_responses().is_some_and(|r| !r.is_empty()))
                .collect()
        } else {
            Vec::new()
        };

    let total_count = own_services.len().saturating_add(parent_services.len());

    if total_count > 0 {
        let detail_section = build_pos_responses_table_section(&own_services, &parent_services);

        b.push_service_list_header(
            depth,
            format!("Pos-Responses ({total_count})"),
            false,
            true,
            vec![detail_section],
            crate::tree::ServiceListType::PosResponses,
        );

        for ds in &own_services {
            add_pos_response_service(b, ds, depth, None);
        }

        for (ds, source_layer_name) in &parent_services {
            add_pos_response_service(b, ds, depth, Some(source_layer_name.clone()));
        }
    }
}

/// Add a single service with negative responses to the tree
fn add_neg_response_service(
    b: &mut TreeBuilder,
    ds: &DiagService<'_>,
    depth: usize,
    source_layer: Option<String>,
) {
    let Some(display_name) = format_service_display_name(ds) else {
        return;
    };

    let sections = build_neg_response_view_sections(ds, source_layer);
    let has_params = ds.neg_responses().is_some_and(|r| {
        r.iter()
            .any(|resp| resp.params().is_some_and(|p| !p.is_empty()))
    });

    b.push_details_structured(
        depth.saturating_add(1),
        display_name,
        false,
        has_params,
        sections,
        NodeType::NegResponse,
    );

    let response_count = ds.neg_responses().map_or(0, |r| r.len());
    for (resp_idx, resp) in ds
        .neg_responses()
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
                param_name,
                param_detail,
                NodeType::Default,
                param.id(),
            );
        }
    }
}

/// Add negative responses section to the tree
pub fn add_neg_responses_section<'a>(
    b: &mut TreeBuilder,
    layer: &DiagLayer<'a>,
    depth: usize,
    variant_parent_refs: Option<impl Iterator<Item = ParentRef<'a>> + 'a>,
) {
    let own_services: Vec<DiagService<'_>> = layer
        .diag_services()
        .map(|services| {
            services
                .iter()
                .map(DiagService)
                .filter(|ds| ds.neg_responses().is_some_and(|r| !r.is_empty()))
                .collect()
        })
        .unwrap_or_default();

    let parent_services: Vec<(DiagService<'_>, String)> =
        if let Some(parent_refs) = variant_parent_refs {
            get_parent_ref_services_recursive(parent_refs)
                .into_iter()
                .filter(|(ds, _)| ds.neg_responses().is_some_and(|r| !r.is_empty()))
                .collect()
        } else {
            Vec::new()
        };

    let total_count = own_services.len().saturating_add(parent_services.len());

    if total_count > 0 {
        let detail_section = build_neg_responses_table_section(&own_services, &parent_services);

        b.push_service_list_header(
            depth,
            format!("Neg-Responses ({total_count})"),
            false,
            true,
            vec![detail_section],
            crate::tree::ServiceListType::NegResponses,
        );

        for ds in &own_services {
            add_neg_response_service(b, ds, depth, None);
        }

        for (ds, source_layer_name) in &parent_services {
            add_neg_response_service(b, ds, depth, Some(source_layer_name.clone()));
        }
    }
}

/// Build complete service view with Pos-Response tabs (used by Pos-Responses section)
fn build_pos_response_view_sections(
    ds: &DiagService<'_>,
    parent_layer_name: Option<String>,
) -> Vec<DetailSectionData> {
    let mut sections = Vec::new();

    // Add header section
    let service_name = ds.diag_comm().and_then(|dc| dc.short_name()).unwrap_or("?");
    let sid = format_service_id(ds);
    let header_title = if sid.is_empty() {
        format!("Pos Response - {service_name}")
    } else {
        format!("Pos Response - {sid} - {service_name}")
    };

    sections.push(DetailSectionData {
        title: header_title,
        render_as_header: true,
        section_type: DetailSectionType::Header,
        content: DetailContent::PlainText(vec![]),
    });

    // Overview
    sections.push(build_overview_section(ds, parent_layer_name));

    // Pos-Response tabs - use rendering logic from this module
    sections.extend(build_pos_responses_sections(ds));

    sections
}

/// Build complete service view with Neg-Response tabs (used by Neg-Responses section)
fn build_neg_response_view_sections(
    ds: &DiagService<'_>,
    parent_layer_name: Option<String>,
) -> Vec<DetailSectionData> {
    let mut sections = Vec::new();

    // Add header section
    let service_name = ds.diag_comm().and_then(|dc| dc.short_name()).unwrap_or("?");
    let sid = format_service_id(ds);
    let header_title = if sid.is_empty() {
        format!("Neg Response - {service_name}")
    } else {
        format!("Neg Response - {sid} - {service_name}")
    };

    sections.push(DetailSectionData {
        title: header_title,
        render_as_header: true,
        section_type: DetailSectionType::Header,
        content: DetailContent::PlainText(vec![]),
    });

    // Overview
    sections.push(build_overview_section(ds, parent_layer_name));

    // Neg-Response tabs - use rendering logic from this module
    sections.extend(build_neg_responses_sections(ds));

    sections
}

/// Build overview section (shared by both pos and neg response views)
fn build_overview_section(
    ds: &DiagService<'_>,
    parent_layer_name: Option<String>,
) -> DetailSectionData {
    let header = DetailRow {
        cells: vec!["Property".to_owned(), "Value".to_owned()],
        cell_types: vec![CellType::Text, CellType::Text],
        indent: 0,
        ..Default::default()
    };

    let mut rows = Vec::new();

    rows.extend(
        ds.diag_comm()
            .and_then(|dc| dc.short_name())
            .map(|sn| DetailRow {
                cells: vec!["Service".to_owned(), sn.to_owned()],
                cell_types: vec![CellType::Text, CellType::Text],
                indent: 0,
                ..Default::default()
            }),
    );
    rows.extend(
        ds.diag_comm()
            .and_then(|dc| dc.semantic())
            .map(|semantic| DetailRow {
                cells: vec!["Semantic".to_owned(), semantic.to_owned()],
                cell_types: vec![CellType::Text, CellType::Text],
                indent: 0,
                ..Default::default()
            }),
    );
    if let Some(sid) = ds.request_id() {
        rows.push(DetailRow {
            cells: vec!["SID".to_owned(), format!("0x{sid:02X}")],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            ..Default::default()
        });
    }
    if let Some((sub_fn, bit_len)) = ds.request_sub_function_id() {
        rows.push(DetailRow {
            cells: vec![
                "Sub-Function".to_owned(),
                format!("0x{sub_fn:04X} ({bit_len} bits)"),
            ],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            ..Default::default()
        });
    }
    rows.push(DetailRow {
        cells: vec!["Addressing".to_owned(), format!("{:?}", ds.addressing())],
        cell_types: vec![CellType::Text, CellType::Text],
        indent: 0,
        ..Default::default()
    });
    rows.push(DetailRow {
        cells: vec![
            "Transmission".to_owned(),
            format!("{:?}", ds.transmission_mode()),
        ],
        cell_types: vec![CellType::Text, CellType::Text],
        indent: 0,
        ..Default::default()
    });

    if let Some(parent_name) = parent_layer_name {
        rows.push(DetailRow {
            cells: vec!["Inherited From".to_owned(), parent_name],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            ..Default::default()
        });
    }

    DetailSectionData {
        title: "Overview".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Overview,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(30),
                ColumnConstraint::Percentage(70),
            ],
            use_row_selection: true,
        },
    }
}

/// Helper to build parameter table for responses
fn build_response_param_section<'a, I>(
    title: &str,
    params: I,
    _section_type: DetailSectionType,
) -> DetailSectionData
where
    I: IntoIterator<Item = Parameter<'a>>,
{
    let header = DetailRow {
        cells: vec![
            "Short Name".to_owned(),
            "Byte".to_owned(),
            "Bit".to_owned(),
            "Bit\nLen".to_owned(),
            "Byte\nLen".to_owned(),
            "Value".to_owned(),
            "DOP".to_owned(),
            "Semantic".to_owned(),
        ],
        cell_types: vec![
            CellType::Text,
            CellType::NumericValue,
            CellType::NumericValue,
            CellType::NumericValue,
            CellType::NumericValue,
            CellType::Text,
            CellType::Text,
            CellType::Text,
        ],
        indent: 0,
        ..Default::default()
    };

    let mut rows: Vec<DetailRow> = Vec::new();

    for param in params {
        let name = param.short_name().unwrap_or("?").to_owned();
        let byte_pos = param.byte_position();
        let bit_pos = param.bit_position();
        let bit_len = "-".to_owned();
        let byte_len = "-".to_owned();
        let value = extract_coded_value(&param);
        let dop_name = extract_dop_name(&param);
        let semantic = param.semantic().unwrap_or_default().to_owned();
        let has_dop = !dop_name.is_empty();
        let param_id = param.id();

        rows.push(DetailRow {
            cells: vec![
                name,
                byte_pos.to_string(),
                bit_pos.to_string(),
                bit_len,
                byte_len,
                value,
                dop_name,
                semantic,
            ],
            cell_types: vec![
                CellType::ParameterName,
                CellType::NumericValue,
                CellType::NumericValue,
                CellType::Text,
                CellType::Text,
                CellType::NumericValue,
                if has_dop {
                    CellType::DopReference
                } else {
                    CellType::Text
                },
                CellType::Text,
            ],
            indent: 0,
            row_type: crate::tree::DetailRowType::Normal,
            metadata: Some(crate::tree::RowMetadata::ParameterRow { param_id }),
        });
    }

    DetailSectionData {
        title: title.to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::PosResponses,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(45),
                ColumnConstraint::Fixed(4),
                ColumnConstraint::Fixed(3),
                ColumnConstraint::Fixed(4),
                ColumnConstraint::Fixed(5),
                ColumnConstraint::Percentage(15),
                ColumnConstraint::Percentage(15),
                ColumnConstraint::Percentage(25),
            ],
            use_row_selection: false,
        },
    }
}

/// Build a table section for the Pos-Responses header showing all services
fn build_pos_responses_table_section(
    own_services: &[DiagService<'_>],
    parent_services: &[(DiagService<'_>, String)],
) -> DetailSectionData {
    let header = DetailRow {
        cells: vec![
            "Short Name".to_owned(),
            "ID".to_owned(),
            "Inherited".to_owned(),
        ],
        cell_types: vec![CellType::Text, CellType::Text, CellType::Text],
        indent: 0,
        ..Default::default()
    };

    let mut rows = Vec::new();

    let build_row = |ds: &DiagService<'_>, inherited: &str| -> Option<DetailRow> {
        let name = ds.diag_comm()?.short_name().unwrap_or("?").to_owned();
        let id_str = format_service_id(ds);
        let id = if id_str.is_empty() {
            "-".to_owned()
        } else {
            id_str
        };
        Some(DetailRow {
            cells: vec![name, id, inherited.to_owned()],
            cell_types: vec![CellType::Text, CellType::Text, CellType::Text],
            indent: 0,
            ..Default::default()
        })
    };

    rows.extend(own_services.iter().filter_map(|ds| build_row(ds, "false")));
    rows.extend(
        parent_services
            .iter()
            .filter_map(|(ds, _)| build_row(ds, "true")),
    );

    let total_count = own_services.len().saturating_add(parent_services.len());

    DetailSectionData {
        title: format!("Pos-Responses ({total_count})"),
        render_as_header: false,
        section_type: DetailSectionType::PosResponses,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(60),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(20),
            ],
            use_row_selection: true,
        },
    }
}

/// Build a table section for the Neg-Responses header showing all services
fn build_neg_responses_table_section(
    own_services: &[DiagService<'_>],
    parent_services: &[(DiagService<'_>, String)],
) -> DetailSectionData {
    let header = DetailRow {
        cells: vec![
            "Short Name".to_owned(),
            "ID".to_owned(),
            "Inherited".to_owned(),
        ],
        cell_types: vec![CellType::Text, CellType::Text, CellType::Text],
        indent: 0,
        ..Default::default()
    };

    let mut rows = Vec::new();

    let build_row = |ds: &DiagService<'_>, inherited: &str| -> Option<DetailRow> {
        let name = ds.diag_comm()?.short_name().unwrap_or("?").to_owned();
        let id_str = format_service_id(ds);
        let id = if id_str.is_empty() {
            "-".to_owned()
        } else {
            id_str
        };
        Some(DetailRow {
            cells: vec![name, id, inherited.to_owned()],
            cell_types: vec![CellType::Text, CellType::Text, CellType::Text],
            indent: 0,
            ..Default::default()
        })
    };

    rows.extend(own_services.iter().filter_map(|ds| build_row(ds, "false")));
    rows.extend(
        parent_services
            .iter()
            .filter_map(|(ds, _)| build_row(ds, "true")),
    );

    let total_count = own_services.len().saturating_add(parent_services.len());

    DetailSectionData {
        title: format!("Neg-Responses ({total_count})"),
        render_as_header: false,
        section_type: DetailSectionType::NegResponses,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(60),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(20),
            ],
            use_row_selection: true,
        },
    }
}

/// Build detail sections for a single parameter
fn build_param_detail_sections(param: &Parameter<'_>) -> Vec<DetailSectionData> {
    let mut sections = Vec::new();

    // Header section
    let param_name = param.short_name().unwrap_or("?");
    sections.push(DetailSectionData {
        title: format!("Parameter - {param_name}"),
        render_as_header: true,
        section_type: DetailSectionType::Header,
        content: DetailContent::PlainText(vec![]),
    });

    // Overview section
    let mut overview_rows = Vec::new();

    if let Some(short_name) = param.short_name() {
        overview_rows.push(DetailRow::normal(
            vec!["Short Name".to_owned(), short_name.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ));
    }

    if let Ok(param_type) = param.param_type() {
        use cda_database::datatypes::ParamType;
        let param_type_str = match param_type {
            ParamType::CodedConst => "CodedConst",
            ParamType::PhysConst => "PhysConst",
            ParamType::Value => "Value",
            _ => "Other",
        };
        overview_rows.push(DetailRow::normal(
            vec!["Type".to_owned(), param_type_str.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ));
    }

    if let Some(semantic) = param.semantic() {
        overview_rows.push(DetailRow::normal(
            vec!["Semantic".to_owned(), semantic.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ));
    }

    let byte_pos = param.byte_position();
    if byte_pos != 0 {
        overview_rows.push(DetailRow::normal(
            vec!["Byte Position".to_owned(), byte_pos.to_string()],
            vec![CellType::Text, CellType::NumericValue],
            0,
        ));
    }

    let bit_pos = param.bit_position();
    if bit_pos != 255 {
        // 255 is the default/unset value
        overview_rows.push(DetailRow::normal(
            vec!["Bit Position".to_owned(), bit_pos.to_string()],
            vec![CellType::Text, CellType::NumericValue],
            0,
        ));
    }

    // Add coded value if it's a CodedConst
    let coded_value = extract_coded_value(param);
    if !coded_value.is_empty() {
        overview_rows.push(DetailRow::normal(
            vec!["Coded Value".to_owned(), coded_value],
            vec![CellType::Text, CellType::NumericValue],
            0,
        ));
    }

    // Add DOP name if available
    let dop_name = extract_dop_name(param);
    if !dop_name.is_empty() {
        overview_rows.push(DetailRow::normal(
            vec!["DOP".to_owned(), dop_name],
            vec![CellType::Text, CellType::DopReference],
            0,
        ));
    }

    if !overview_rows.is_empty() {
        let header = DetailRow::header(
            vec!["Property".to_owned(), "Value".to_owned()],
            vec![CellType::Text, CellType::Text],
        );

        sections.push(
            DetailSectionData::new(
                "Overview".to_owned(),
                DetailContent::Table {
                    header,
                    rows: overview_rows,
                    constraints: vec![
                        ColumnConstraint::Percentage(40),
                        ColumnConstraint::Percentage(60),
                    ],
                    use_row_selection: true,
                },
                false,
            )
            .with_type(DetailSectionType::Overview),
        );
    }

    sections
}
