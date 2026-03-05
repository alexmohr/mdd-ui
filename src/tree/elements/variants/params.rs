/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::{DiagService, Parameter};

use super::{
    dops::parse_dop_name,
    format_service_id,
    services::{extract_coded_value, extract_dop_name},
};
use crate::tree::types::{
    BIT_POSITION_UNSET, CellJumpTarget, CellJumpTargetType, CellType, ColumnConstraint,
    DetailContent, DetailRow, DetailSectionData, DetailSectionType,
};

/// Build a key-value row for a DOP reference that is navigable via Enter.
/// Uses the parsed display name when available so the navigation target
/// matches the tree node label produced by `add_dops_section`.
fn dop_kv_row(dop_name: &str) -> DetailRow {
    let parsed = parse_dop_name(dop_name);
    let display = parsed.display_name();
    let nav_name = if display.is_empty() {
        dop_name.to_owned()
    } else {
        display
    };
    DetailRow::with_jump_targets(
        vec!["DOP".to_owned(), nav_name.clone()],
        vec![CellType::Text, CellType::DopReference],
        vec![
            None,
            Some(CellJumpTarget::new(CellJumpTargetType::Dop { name: nav_name })),
        ],
        0,
    )
}

/// Format a `ParamType` value as a static label.
fn param_type_label(pt: &cda_database::datatypes::ParamType) -> &'static str {
    use cda_database::datatypes::ParamType;
    match pt {
        ParamType::CodedConst => "CodedConst",
        ParamType::Dynamic => "Dynamic",
        ParamType::LengthKey => "LengthKey",
        ParamType::MatchingRequestParam => "MatchingRequestParam",
        ParamType::NrcConst => "NrcConst",
        ParamType::PhysConst => "PhysConst",
        ParamType::Reserved => "Reserved",
        ParamType::System => "System",
        ParamType::TableEntry => "TableEntry",
        ParamType::TableKey => "TableKey",
        ParamType::TableStruct => "TableStruct",
        ParamType::Value => "Value",
    }
}

/// Append detail rows for a `DiagCodedType` flatbuf value.  Implemented as a
/// macro because the underlying flatbuf types are crate-private and cannot be
/// named in function signatures.
macro_rules! append_dct_rows {
    ($dct:expr, $rows:expr) => {{
        let dct = &$dct;
        $rows.push(DetailRow::normal(
            vec!["DCT Type".to_owned(), format!("{:?}", dct.type_())],
            vec![CellType::Text, CellType::Text],
            0,
        ));

        if let Some(enc) = dct.base_type_encoding() {
            $rows.push(DetailRow::normal(
                vec!["Base Type Encoding".to_owned(), enc.to_owned()],
                vec![CellType::Text, CellType::Text],
                0,
            ));
        }

        $rows.push(DetailRow::normal(
            vec![
                "Base Data Type".to_owned(),
                format!("{:?}", dct.base_data_type()),
            ],
            vec![CellType::Text, CellType::Text],
            0,
        ));

        $rows.push(DetailRow::normal(
            vec![
                "High-Low Byte Order".to_owned(),
                dct.is_high_low_byte_order().to_string(),
            ],
            vec![CellType::Text, CellType::Text],
            0,
        ));

        let length_type = format!("{:?}", dct.specific_data_type());
        $rows.push(DetailRow::normal(
            vec!["Length Type".to_owned(), length_type],
            vec![CellType::Text, CellType::Text],
            0,
        ));

        if let Some(slt) = dct.specific_data_as_standard_length_type() {
            $rows.push(DetailRow::normal(
                vec!["Bit Length".to_owned(), slt.bit_length().to_string()],
                vec![CellType::Text, CellType::NumericValue],
                0,
            ));
            let mask_str = slt
                .bit_mask()
                .map_or_else(|| "None".to_owned(), |m| format!("{m:?}"));
            $rows.push(DetailRow::normal(
                vec!["Bit Mask".to_owned(), mask_str],
                vec![CellType::Text, CellType::Text],
                0,
            ));
            $rows.push(DetailRow::normal(
                vec!["Condensed".to_owned(), slt.condensed().to_string()],
                vec![CellType::Text, CellType::Text],
                0,
            ));
        } else if let Some(mml) = dct.specific_data_as_min_max_length_type() {
            $rows.push(DetailRow::normal(
                vec!["Min Length".to_owned(), mml.min_length().to_string()],
                vec![CellType::Text, CellType::NumericValue],
                0,
            ));
            let max = mml
                .max_length()
                .map_or_else(|| "None".to_owned(), |v| v.to_string());
            $rows.push(DetailRow::normal(
                vec!["Max Length".to_owned(), max],
                vec![CellType::Text, CellType::NumericValue],
                0,
            ));
            $rows.push(DetailRow::normal(
                vec!["Termination".to_owned(), format!("{:?}", mml.termination())],
                vec![CellType::Text, CellType::Text],
                0,
            ));
        } else if let Some(lli) = dct.specific_data_as_leading_length_info_type() {
            $rows.push(DetailRow::normal(
                vec!["Bit Length".to_owned(), lli.bit_length().to_string()],
                vec![CellType::Text, CellType::NumericValue],
                0,
            ));
        } else if let Some(pli) = dct.specific_data_as_param_length_info_type() {
            let key_name = pli.length_key().and_then(|p| p.short_name()).unwrap_or("-");
            $rows.push(DetailRow::normal(
                vec!["Length Key Param".to_owned(), key_name.to_owned()],
                vec![CellType::Text, CellType::Text],
                0,
            ));
        }
    }};
}

/// Build detail sections for a single parameter (Overview with key-value
/// properties).  Shared by both request and response parameter views.
pub fn build_param_detail_sections(param: &Parameter<'_>) -> Vec<DetailSectionData> {
    let mut sections = Vec::new();

    let param_name = param.short_name().unwrap_or("?");
    sections.push(DetailSectionData {
        title: format!("Parameter - {param_name}"),
        render_as_header: true,
        section_type: DetailSectionType::Header,
        content: DetailContent::PlainText(vec![]),
    });

    let mut overview_rows = Vec::new();

    // ID
    overview_rows.push(DetailRow::normal(
        vec!["ID".to_owned(), param.id().to_string()],
        vec![CellType::Text, CellType::NumericValue],
        0,
    ));

    if let Some(short_name) = param.short_name() {
        overview_rows.push(DetailRow::normal(
            vec!["Short Name".to_owned(), short_name.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ));
    }

    if let Ok(param_type) = param.param_type() {
        overview_rows.push(DetailRow::normal(
            vec!["Type".to_owned(), param_type_label(&param_type).to_owned()],
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

    // Always show byte/bit position
    overview_rows.push(DetailRow::normal(
        vec![
            "Byte Position".to_owned(),
            param.byte_position().to_string(),
        ],
        vec![CellType::Text, CellType::NumericValue],
        0,
    ));

    let bit_pos = param.bit_position();
    overview_rows.push(DetailRow::normal(
        vec![
            "Bit Position".to_owned(),
            if bit_pos == BIT_POSITION_UNSET {
                "unset".to_owned()
            } else {
                bit_pos.to_string()
            },
        ],
        vec![CellType::Text, CellType::NumericValue],
        0,
    ));

    if let Some(pdv) = param.physical_default_value() {
        overview_rows.push(DetailRow::normal(
            vec!["Physical Default Value".to_owned(), pdv.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ));
    }

    // Coded value (hex formatted, for CodedConst)
    let coded_value = extract_coded_value(param);
    if !coded_value.is_empty() {
        overview_rows.push(DetailRow::normal(
            vec!["Coded Value".to_owned(), coded_value],
            vec![CellType::Text, CellType::NumericValue],
            0,
        ));
    }

    // DOP reference (for Value type)
    let dop_name = extract_dop_name(param);
    if !dop_name.is_empty() {
        overview_rows.push(dop_kv_row(&dop_name));
    }

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

    // Type-specific section
    let specific_rows = build_specific_data_rows(param);
    if !specific_rows.is_empty() {
        let title = param.param_type().map_or_else(
            |_| "Specific Data".to_owned(),
            |pt| param_type_label(&pt).to_owned(),
        );

        let header = DetailRow::header(
            vec!["Property".to_owned(), "Value".to_owned()],
            vec![CellType::Text, CellType::Text],
        );

        sections.push(
            DetailSectionData::new(
                title,
                DetailContent::Table {
                    header,
                    rows: specific_rows,
                    constraints: vec![
                        ColumnConstraint::Percentage(40),
                        ColumnConstraint::Percentage(60),
                    ],
                    use_row_selection: true,
                },
                false,
            )
            .with_type(DetailSectionType::Custom),
        );
    }

    sections
}

/// Build rows for the type-specific data of a parameter.
fn build_specific_data_rows(param: &Parameter<'_>) -> Vec<DetailRow> {
    let mut rows = Vec::new();

    // CodedConst
    if let Some(cc) = param.specific_data_as_coded_const() {
        if let Some(cv) = cc.coded_value() {
            rows.push(DetailRow::normal(
                vec!["Coded Value".to_owned(), cv.to_owned()],
                vec![CellType::Text, CellType::Text],
                0,
            ));
        }
        if let Some(dct) = cc.diag_coded_type() {
            append_dct_rows!(dct, rows);
        }
        return rows;
    }

    // NrcConst
    if let Some(nrc) = param.specific_data_as_nrc_const() {
        if let Some(vals) = nrc.coded_values() {
            let values_str: Vec<&str> = vals.iter().collect();
            rows.push(DetailRow::normal(
                vec!["Coded Values".to_owned(), values_str.join(", ")],
                vec![CellType::Text, CellType::Text],
                0,
            ));
        }
        if let Some(dct) = nrc.diag_coded_type() {
            append_dct_rows!(dct, rows);
        }
        return rows;
    }

    build_remaining_specific_rows(param)
}

/// Build type-specific rows for param types that do not use `DiagCodedType`.
fn build_remaining_specific_rows(param: &Parameter<'_>) -> Vec<DetailRow> {
    let mut rows = Vec::new();

    // MatchingRequestParam
    if let Some(mrp) = param.specific_data_as_matching_request_param() {
        rows.push(DetailRow::normal(
            vec![
                "Request Byte Pos".to_owned(),
                mrp.request_byte_pos().to_string(),
            ],
            vec![CellType::Text, CellType::NumericValue],
            0,
        ));
        rows.push(DetailRow::normal(
            vec!["Byte Length".to_owned(), mrp.byte_length().to_string()],
            vec![CellType::Text, CellType::NumericValue],
            0,
        ));
        return rows;
    }

    // PhysConst
    if let Some(pc) = param.specific_data_as_phys_const() {
        if let Some(v) = pc.phys_constant_value() {
            rows.push(DetailRow::normal(
                vec!["Phys Constant Value".to_owned(), v.to_owned()],
                vec![CellType::Text, CellType::Text],
                0,
            ));
        }
        if let Some(dop) = pc.dop() {
            rows.push(dop_kv_row(dop.short_name().unwrap_or("-")));
        }
        return rows;
    }

    // Reserved
    if let Some(res) = param.specific_data_as_reserved() {
        rows.push(DetailRow::normal(
            vec!["Bit Length".to_owned(), res.bit_length().to_string()],
            vec![CellType::Text, CellType::NumericValue],
            0,
        ));
        return rows;
    }

    // Value
    if let Some(val) = param.specific_data_as_value() {
        if let Some(pdv) = val.physical_default_value() {
            rows.push(DetailRow::normal(
                vec!["Physical Default Value".to_owned(), pdv.to_owned()],
                vec![CellType::Text, CellType::Text],
                0,
            ));
        }
        if let Some(dop) = val.dop() {
            rows.push(dop_kv_row(dop.short_name().unwrap_or("-")));
        }
        return rows;
    }

    // System
    if let Some(sys) = param.specific_data_as_system() {
        if let Some(sp) = sys.sys_param() {
            rows.push(DetailRow::normal(
                vec!["Sys Param".to_owned(), sp.to_owned()],
                vec![CellType::Text, CellType::Text],
                0,
            ));
        }
        if let Some(dop) = sys.dop() {
            rows.push(dop_kv_row(dop.short_name().unwrap_or("-")));
        }
        return rows;
    }

    // LengthKeyRef
    if let Some(lkr) = param.specific_data_as_length_key_ref() {
        if let Some(dop) = lkr.dop() {
            rows.push(dop_kv_row(dop.short_name().unwrap_or("-")));
        }
        return rows;
    }

    // TableEntry
    if let Some(te) = param.specific_data_as_table_entry() {
        rows.push(DetailRow::normal(
            vec!["Target".to_owned(), format!("{:?}", te.target())],
            vec![CellType::Text, CellType::Text],
            0,
        ));
        if let Some(p) = te.param() {
            rows.push(DetailRow::normal(
                vec![
                    "Entry Param".to_owned(),
                    p.short_name().unwrap_or("-").to_owned(),
                ],
                vec![CellType::Text, CellType::Text],
                0,
            ));
        }
        if let Some(tr) = te.table_row()
            && let Some(sn) = tr.short_name()
        {
            rows.push(DetailRow::normal(
                vec!["Table Row".to_owned(), sn.to_owned()],
                vec![CellType::Text, CellType::Text],
                0,
            ));
        }
        return rows;
    }

    // TableStruct
    if let Some(ts) = param.specific_data_as_table_struct() {
        if let Some(key) = ts.table_key() {
            rows.push(DetailRow::normal(
                vec![
                    "Table Key Param".to_owned(),
                    key.short_name().unwrap_or("-").to_owned(),
                ],
                vec![CellType::Text, CellType::Text],
                0,
            ));
        }
        return rows;
    }

    rows
}

/// Build a parameter table section (the column-based param list used by
/// request / response detail views).  `section_type` distinguishes Requests
/// from `PosResponses` / `NegResponses`.
pub fn build_param_section<'a, I>(
    title: &str,
    params: I,
    section_type: DetailSectionType,
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

    let rows: Vec<DetailRow> = params
        .into_iter()
        .map(|param| {
            let name = param.short_name().unwrap_or("?").to_owned();
            let byte_pos = param.byte_position();
            let bit_pos = param.bit_position();
            let value = extract_coded_value(&param);
            let dop_name = extract_dop_name(&param);
            let semantic = param.semantic().unwrap_or_default().to_owned();
            let has_dop = !dop_name.is_empty();
            let param_id = param.id();

            let dop_jump = if has_dop {
                Some(CellJumpTarget::new(CellJumpTargetType::Dop {
                    name: dop_name.clone(),
                }))
            } else {
                None
            };

            DetailRow {
                cells: vec![
                    name,
                    byte_pos.to_string(),
                    bit_pos.to_string(),
                    "-".to_owned(),
                    "-".to_owned(),
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
                    Some(CellJumpTarget::new(CellJumpTargetType::Parameter {
                        param_id,
                    })),
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
                diff_status: None,
            }
        })
        .collect();

    DetailSectionData {
        title: title.to_owned(),
        render_as_header: false,
        section_type,
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

/// Build a service-list table section (the header table showing all services
/// with Short Name / ID / Inherited columns).  Used by both the Requests and
/// Responses list headers.
pub fn build_service_list_table_section(
    own_services: &[DiagService<'_>],
    parent_services: &[(DiagService<'_>, String)],
    label: &str,
    section_type: DetailSectionType,
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
            cell_types: vec![CellType::ParameterName, CellType::Text, CellType::Text],
            cell_jump_targets: vec![
                Some(CellJumpTarget::new(CellJumpTargetType::TreeNodeByName)),
                None,
                None,
            ],
            indent: 0,
            ..Default::default()
        })
    };

    let mut rows = Vec::new();
    rows.extend(own_services.iter().filter_map(|ds| build_row(ds, "false")));
    rows.extend(
        parent_services
            .iter()
            .filter_map(|(ds, _)| build_row(ds, "true")),
    );

    let total_count = own_services.len().saturating_add(parent_services.len());

    DetailSectionData {
        title: format!("{label} ({total_count})"),
        render_as_header: false,
        section_type,
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
