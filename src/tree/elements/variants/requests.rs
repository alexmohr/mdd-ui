/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::{DiagLayer, DiagService, Parameter, ParentRef};

use super::services::{extract_coded_value, extract_dop_name, get_parent_ref_services_recursive};
use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ColumnConstraint, DetailContent, DetailRow, DetailSectionData, DetailSectionType,
        NodeType,
    },
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
        // Build detail section for Requests header showing all services in a table
        let detail_section = build_requests_table_section(&own_services, &parent_services);

        b.push_service_list_header(
            depth,
            format!("Requests ({total_count})"),
            false,
            true,
            vec![detail_section],
            crate::tree::ServiceListType::Requests,
        );

        // Add own services first - using EXACTLY the same display as DiagComm
        for ds in &own_services {
            let Some(display_name) = super::format_service_display_name(ds) else {
                continue;
            };

            // Build full service details, but with Request tab rendered by this module
            let sections = build_request_view_sections(ds, None);

            // Check if there are params to show as children
            let has_params = ds
                .request()
                .and_then(|req| req.params())
                .is_some_and(|p| !p.is_empty());

            b.push_details_structured(
                depth.saturating_add(1),
                display_name.clone(),
                false,
                has_params,
                sections,
                NodeType::Request, // Use Request node type for navigation
            );

            // Add params as child nodes
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

        // Add parent ref services with different node type (same as DiagComm)
        for (ds, source_layer_name) in &parent_services {
            let Some(display_name) = super::format_service_display_name(ds) else {
                continue;
            };

            // Build full service details, but with Request tab rendered by this module
            let sections = build_request_view_sections(ds, Some(source_layer_name.clone()));

            // Check if there are params to show as children
            let has_params = ds
                .request()
                .and_then(|req| req.params())
                .is_some_and(|p| !p.is_empty());

            b.push_details_structured(
                depth.saturating_add(1),
                display_name.clone(),
                false,
                has_params,
                sections,
                NodeType::Request, // Use Request node type for navigation (inherited)
            );

            // Add params as child nodes
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
    }
}

/// Build the Request tab section - this is the core rendering logic for Request data
/// `DiagComm` module should import and use this function to render the Request tab
pub fn build_request_section(ds: &DiagService<'_>) -> Option<DetailSectionData> {
    let req = ds.request()?;
    let params = req.params().into_iter().flatten().map(Parameter);

    Some(build_request_param_section("Request", params))
}

/// Build complete service view with Request tab (used by Requests section)
fn build_request_view_sections(
    ds: &DiagService<'_>,
    parent_layer_name: Option<String>,
) -> Vec<DetailSectionData> {
    let mut sections = Vec::new();

    // Add header section with service ID and name
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

    // Overview - table with key-value pairs
    let header = DetailRow {
        cells: vec!["Property".to_owned(), "Value".to_owned()],
        cell_types: vec![CellType::Text, CellType::Text],
        indent: 0,
        ..Default::default()
    };

    let mut rows = Vec::new();

    if let Some(dc) = ds.diag_comm() {
        rows.extend(dc.short_name().map(|sn| DetailRow {
            cells: vec!["Service".to_owned(), sn.to_owned()],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            ..Default::default()
        }));
        rows.extend(dc.semantic().map(|semantic| DetailRow {
            cells: vec!["Semantic".to_owned(), semantic.to_owned()],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            ..Default::default()
        }));
    }
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

    // Add inheritance information only if inherited
    if let Some(parent_name) = parent_layer_name {
        rows.push(DetailRow::inherited_from(parent_name));
    }

    sections.push(DetailSectionData {
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
    });

    // Request params - use the rendering logic from this module
    if let Some(request_section) = build_request_section(ds) {
        sections.push(request_section);
    }

    sections
}

/// Helper to build parameter table for requests
fn build_request_param_section<'a, I>(title: &str, params: I) -> DetailSectionData
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

        let dop_jump = if has_dop {
            Some(crate::tree::CellJumpTarget::Dop {
                name: dop_name.clone(),
            })
        } else {
            None
        };

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
            cell_jump_targets: vec![
                Some(crate::tree::CellJumpTarget::Parameter { param_id }),
                None,
                None,
                None,
                None,
                None,
                dop_jump,
                None,
            ],
            indent: 0,
            row_type: crate::tree::DetailRowType::Normal,
            metadata: Some(crate::tree::RowMetadata::ParameterRow { param_id }),
        });
    }

    DetailSectionData {
        title: title.to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Requests,
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
/// Build a table section for the Requests header showing all services
fn build_requests_table_section(
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

    // Helper: build a row for a service with given inherited flag
    let build_row = |ds: &DiagService<'_>, inherited: &str| -> Option<DetailRow> {
        let dc = ds.diag_comm()?;
        let name = dc.short_name().unwrap_or("?").to_owned();
        let id_str = super::format_service_id(ds);
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

    // Add own services first (inherited = false)
    rows.extend(own_services.iter().filter_map(|ds| build_row(ds, "false")));

    // Add parent services (inherited = true)
    rows.extend(
        parent_services
            .iter()
            .filter_map(|(ds, _)| build_row(ds, "true")),
    );

    let total_count = own_services.len().saturating_add(parent_services.len());

    DetailSectionData {
        title: format!("Requests ({total_count})"),
        render_as_header: false,
        section_type: DetailSectionType::Requests,
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
