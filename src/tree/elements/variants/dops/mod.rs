/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

mod dtc_dops;
mod dynamic_length_fields;
mod end_of_pdu;
mod env_data;
mod mux_dops;
mod normal_dops;
mod static_fields;
mod structures;

use cda_database::datatypes::{DiagLayer, DiagService, Parameter};

use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ColumnConstraint, DetailContent, DetailRow, DetailRowType, DetailSectionData,
        DetailSectionType, NodeType,
    },
};

/// Semantic category of a group of DOPs, derived from the `DataOperationVariant`
#[derive(Clone, Copy, PartialEq, Eq)]
enum DopCategory {
    DtcDops,
    EnvDataDescs,
    EnvDatas,
    DataObjectProps,
    Structures,
    StaticFields,
    DynamicLengthFields,
    EndOfPduFields,
    MuxDops,
}

impl DopCategory {
    fn label(self) -> &'static str {
        match self {
            Self::DtcDops => "Dtc Dops",
            Self::EnvDataDescs => "Env Data Descs",
            Self::EnvDatas => "Env Datas",
            Self::DataObjectProps => "Data Object Props",
            Self::Structures => "Structures",
            Self::StaticFields => "Static Fields",
            Self::DynamicLengthFields => "Dynamic Length Fields",
            Self::EndOfPduFields => "End Of Pdu Fields",
            Self::MuxDops => "Mux Dops",
        }
    }

    /// Build detail sections appropriate for this category
    fn build_detail_sections(self, dops: &[DopInfo<'_>]) -> Vec<DetailSectionData> {
        match self {
            Self::DtcDops => dtc_dops::build_dtc_dops_category_sections(dops),
            Self::EnvDataDescs | Self::EnvDatas | Self::DataObjectProps => {
                build_category_overview_table(dops)
            }
            Self::Structures
            | Self::StaticFields
            | Self::DynamicLengthFields
            | Self::EndOfPduFields
            | Self::MuxDops => build_short_name_only_overview(dops),
        }
    }

    /// Whether individual DOPs in this category have tree children
    fn has_dop_children(self) -> bool {
        matches!(self, Self::DtcDops)
    }

    /// Add child tree nodes for an individual DOP in this category
    fn add_dop_children(self, b: &mut TreeBuilder, dop_info: &DopInfo<'_>, depth: usize) {
        if self == Self::DtcDops {
            dtc_dops::add_dtc_dop_children(b, dop_info, depth);
        }
    }
}

impl std::fmt::Display for DopCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Structure to hold DOP information with reference to actual DOP for detail extraction
struct DopInfo<'a> {
    name: String,
    dop_type: String,
    dop: cda_database::datatypes::DataOperation<'a>,
    category: Option<String>,
    internal_unit: Option<String>,
    phys_unit: Option<String>,
    desc_id: Option<String>,
}

/// Parsed information from DOP name (e.g., `IDENTICAL_A_UINT32_0x00_0xFFFFFFFF_MicroSecond`)
#[derive(Default)]
struct ParsedDopName {
    compu_category: Option<String>,
    data_type: Option<String>,
    range_min: Option<String>,
    range_max: Option<String>,
    unit: Option<String>,
}

/// Parse encoded information from DOP short name
fn parse_dop_name(name: &str) -> ParsedDopName {
    let parts: Vec<&str> = name.split('_').collect();
    let mut parsed = ParsedDopName::default();

    if parts.is_empty() {
        return parsed;
    }

    // First part might be compu category (IDENTICAL, LINEAR, TEXT_TABLE, etc.)
    let compu_categories = [
        "IDENTICAL",
        "LINEAR",
        "TEXTTABLE",
        "SCALE",
        "COMPUCODE",
        "TABINTP",
        "RATFUNC",
    ];
    if let Some(&first_part) = parts.first()
        && compu_categories
            .iter()
            .any(|&cat| first_part.starts_with(cat))
    {
        parsed.compu_category = Some(first_part.to_owned());
    }

    // A_UINT32, A_INT32, A_FLOAT32, A_ASCIISTRING, etc.
    for (i, part) in parts.iter().enumerate() {
        if part.starts_with("A_") {
            parsed.data_type = parts
                .get(i.saturating_add(1))
                .map(|next| format!("{part}_{next}"))
                .or_else(|| Some(part.to_string()));
        }
    }

    let hex_parts: Vec<&str> = parts
        .iter()
        .filter(|p| p.starts_with("0x") || p.starts_with("0X"))
        .copied()
        .collect();
    if hex_parts.len() >= 2 {
        parsed.range_min = hex_parts.first().map(|s| (*s).to_owned());
        parsed.range_max = hex_parts.get(1).map(|s| (*s).to_owned());
    } else if hex_parts.len() == 1 {
        parsed.range_min = hex_parts.first().map(|s| (*s).to_owned());
    }

    // Last part might be unit (if not a hex value or data type)
    if let Some(last) = parts.last()
        && !last.starts_with("0x")
        && !last.starts_with("0X")
        && !last.starts_with("A_")
    {
        let units = [
            "Second",
            "MicroSecond",
            "MilliSecond",
            "Meter",
            "KiloMeter",
            "Volt",
            "Ampere",
            "Celsius",
            "Pascal",
        ];
        if units.iter().any(|&u| last.contains(u)) {
            parsed.unit = Some(last.to_string());
        }
    }

    parsed
}

/// Add DOPs section to the tree by collecting from service/job request/response params
pub fn add_dops_section<'a>(b: &mut TreeBuilder, layer: &DiagLayer<'a>, depth: usize) {
    use cda_database::datatypes::DataOperationVariant;

    let mut all_dops: Vec<DopInfo<'a>> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    collect_dops_from_layer(layer, &mut all_dops, &mut seen);

    let unique_dops = all_dops;

    if unique_dops.is_empty() {
        return;
    }
    let mut dtc_dops: Vec<DopInfo> = Vec::new();
    let mut env_data_descs: Vec<DopInfo> = Vec::new();
    let mut env_datas: Vec<DopInfo> = Vec::new();
    let mut data_object_props: Vec<DopInfo> = Vec::new();
    let mut structures: Vec<DopInfo> = Vec::new();
    let mut static_fields: Vec<DopInfo> = Vec::new();
    let mut dynamic_length_fields: Vec<DopInfo> = Vec::new();
    let mut end_of_pdu_fields: Vec<DopInfo> = Vec::new();
    let mut mux_dops: Vec<DopInfo> = Vec::new();

    for dop_info in unique_dops {
        if let Ok(variant) = dop_info.dop.variant() {
            match variant {
                DataOperationVariant::Dtc(_) => dtc_dops.push(dop_info),
                DataOperationVariant::EnvDataDesc(_) => env_data_descs.push(dop_info),
                DataOperationVariant::EnvData(_) => env_datas.push(dop_info),
                DataOperationVariant::Structure(_) => structures.push(dop_info),
                DataOperationVariant::StaticField(_) => static_fields.push(dop_info),
                DataOperationVariant::DynamicLengthField(_) => dynamic_length_fields.push(dop_info),
                DataOperationVariant::EndOfPdu(_) => end_of_pdu_fields.push(dop_info),
                DataOperationVariant::Mux(_) => mux_dops.push(dop_info),
                DataOperationVariant::Normal(_) => data_object_props.push(dop_info),
            }
        } else {
            data_object_props.push(dop_info);
        }
    }

    let categories: Vec<(DopCategory, &[DopInfo])> = [
        (DopCategory::DtcDops, dtc_dops.as_slice()),
        (DopCategory::EnvDataDescs, env_data_descs.as_slice()),
        (DopCategory::EnvDatas, env_datas.as_slice()),
        (DopCategory::DataObjectProps, data_object_props.as_slice()),
        (DopCategory::Structures, structures.as_slice()),
        (DopCategory::StaticFields, static_fields.as_slice()),
        (
            DopCategory::DynamicLengthFields,
            dynamic_length_fields.as_slice(),
        ),
        (DopCategory::EndOfPduFields, end_of_pdu_fields.as_slice()),
        (DopCategory::MuxDops, mux_dops.as_slice()),
    ]
    .into_iter()
    .filter(|(_, dops)| !dops.is_empty())
    .collect();

    let dops_detail = build_dops_overview_table(&categories);

    // DiagDataDictionarySpec section header (yellow)
    b.push_details_structured(
        depth,
        "DiagDataDictionarySpec".to_owned(),
        false,
        true,
        dops_detail,
        NodeType::DOP,
    );

    for (cat, dops) in &categories {
        if !dops.is_empty() {
            let cat_detail = cat.build_detail_sections(dops);

            b.push_details_structured(
                depth.saturating_add(1),
                format!("{} ({})", cat.label(), dops.len()),
                false,
                true,
                cat_detail,
                NodeType::Default,
            );

            let expandable = cat.has_dop_children();
            for dop_info in *dops {
                let detail_sections = build_dop_detail_sections(dop_info);
                b.push_details_structured(
                    depth.saturating_add(2),
                    dop_info.name.clone(),
                    false,
                    expandable,
                    detail_sections,
                    NodeType::Default,
                );

                if expandable {
                    cat.add_dop_children(b, dop_info, depth.saturating_add(3));
                }
            }
        }
    }
}

/// Push a DOP to the collection and recursively collect nested DOPs
fn collect_single_dop<'a>(
    dop_wrap: cda_database::datatypes::DataOperation<'a>,
    all_dops: &mut Vec<DopInfo<'a>>,
    seen: &mut std::collections::HashSet<String>,
) {
    let Some(name) = dop_wrap.short_name() else {
        return;
    };
    if !seen.insert(name.to_owned()) {
        return;
    }

    let specific_type_raw = format!("{:?}", dop_wrap.specific_data_type());
    let dop_type = specific_type_raw.trim_matches('"').to_owned();

    let (category, internal_unit, phys_unit, desc_id) = extract_dop_metadata(&dop_wrap);

    collect_nested_dops(&dop_wrap, all_dops, seen);

    all_dops.push(DopInfo {
        name: name.to_owned(),
        dop_type,
        dop: dop_wrap,
        category,
        internal_unit,
        phys_unit,
        desc_id,
    });
}

/// Extract metadata (category, units, `desc_id`) from a `DataOperation`
fn extract_dop_metadata(
    dop_wrap: &cda_database::datatypes::DataOperation<'_>,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    use cda_database::datatypes::DataOperationVariant;
    let Ok(variant) = dop_wrap.variant() else {
        return (None, None, None, None);
    };
    match variant {
        DataOperationVariant::Normal(normal_dop) => {
            let cat = normal_dop
                .compu_method()
                .map(|cm| format!("{:?}", cm.category()));
            let int_unit = normal_dop
                .unit_ref()
                .and_then(|u| u.short_name())
                .map(std::borrow::ToOwned::to_owned);
            let phys = normal_dop
                .unit_ref()
                .and_then(|u| u.display_name())
                .or_else(|| normal_dop.unit_ref().and_then(|u| u.short_name()))
                .map(std::borrow::ToOwned::to_owned);
            (cat, int_unit, phys, None)
        }
        DataOperationVariant::EnvDataDesc(env_desc) => {
            let did = env_desc
                .param_short_name()
                .map(std::borrow::ToOwned::to_owned);
            (None, None, None, did)
        }
        DataOperationVariant::EndOfPdu(_)
        | DataOperationVariant::Structure(_)
        | DataOperationVariant::EnvData(_)
        | DataOperationVariant::Dtc(_)
        | DataOperationVariant::StaticField(_)
        | DataOperationVariant::Mux(_)
        | DataOperationVariant::DynamicLengthField(_) => (None, None, None, None),
    }
}

/// Recursively collect nested DOPs from compound DOP types.
/// Uses raw `FlatBuffer` accessors (via .0) to preserve the buffer lifetime 'a.
fn collect_nested_dops<'a>(
    dop_wrap: &cda_database::datatypes::DataOperation<'a>,
    all_dops: &mut Vec<DopInfo<'a>>,
    seen: &mut std::collections::HashSet<String>,
) {
    // Access the inner raw DOP directly to get proper 'a lifetimes
    // (the wrapper's variant() method has restrictive lifetime bounds)
    let raw = dop_wrap.0;

    macro_rules! collect_field_dops {
        ($field:expr) => {
            $field
                .basic_structure()
                .into_iter()
                .chain($field.env_data_desc())
                .for_each(|d| {
                    collect_single_dop(cda_database::datatypes::DataOperation(d), all_dops, seen)
                })
        };
    }

    raw.specific_data_as_structure()
        .and_then(|s| s.params())
        .into_iter()
        .flat_map(|p| p.iter())
        .for_each(|param| collect_dop_from_param(&Parameter(param), all_dops, seen));

    raw.specific_data_as_env_data_desc()
        .and_then(|ed| ed.env_datas())
        .into_iter()
        .flat_map(|v| v.iter())
        .for_each(|env_dop| {
            collect_single_dop(
                cda_database::datatypes::DataOperation(env_dop),
                all_dops,
                seen,
            );
        });

    raw.specific_data_as_env_data()
        .and_then(|ed| ed.params())
        .into_iter()
        .flat_map(|p| p.iter())
        .for_each(|param| collect_dop_from_param(&Parameter(param), all_dops, seen));

    if let Some(mux_dop) = raw.specific_data_as_muxdop() {
        mux_dop
            .switch_key()
            .and_then(|sk| sk.dop())
            .into_iter()
            .chain(mux_dop.default_case().and_then(|dc| dc.structure()))
            .for_each(|d| {
                collect_single_dop(cda_database::datatypes::DataOperation(d), all_dops, seen);
            });

        mux_dop
            .cases()
            .into_iter()
            .flat_map(|c| c.iter())
            .filter_map(|case| case.structure())
            .for_each(|d| {
                collect_single_dop(cda_database::datatypes::DataOperation(d), all_dops, seen);
            });
    }

    [
        raw.specific_data_as_static_field()
            .and_then(|sf| sf.field()),
        raw.specific_data_as_end_of_pdu_field()
            .and_then(|ep| ep.field()),
        raw.specific_data_as_dynamic_length_field()
            .and_then(|df| df.field()),
    ]
    .into_iter()
    .flatten()
    .for_each(|field| collect_field_dops!(field));

    raw.specific_data_as_dynamic_length_field()
        .and_then(|df| df.determine_number_of_items())
        .and_then(|det| det.dop())
        .into_iter()
        .for_each(|d| {
            collect_single_dop(cda_database::datatypes::DataOperation(d), all_dops, seen);
        });
}

/// Extract a DOP from a Parameter wrapper and add it to the collection
fn collect_dop_from_param<'a>(
    param: &Parameter<'a>,
    all_dops: &mut Vec<DopInfo<'a>>,
    seen: &mut std::collections::HashSet<String>,
) {
    use cda_database::datatypes::ParamType;

    let Ok(param_type) = param.param_type() else {
        return;
    };

    macro_rules! collect_table_row {
        ($row:expr) => {
            $row.dop()
                .into_iter()
                .chain($row.structure())
                .for_each(|d| {
                    collect_single_dop(cda_database::datatypes::DataOperation(d), all_dops, seen)
                })
        };
    }

    match param_type {
        ParamType::Value => param.specific_data_as_value().and_then(|v| v.dop()),
        ParamType::PhysConst => param.specific_data_as_phys_const().and_then(|v| v.dop()),
        ParamType::LengthKey => param
            .specific_data_as_length_key_ref()
            .and_then(|v| v.dop()),
        ParamType::System => param.specific_data_as_system().and_then(|v| v.dop()),
        ParamType::CodedConst
        | ParamType::Dynamic
        | ParamType::MatchingRequestParam
        | ParamType::NrcConst
        | ParamType::Reserved
        | ParamType::TableEntry
        | ParamType::TableKey
        | ParamType::TableStruct => None,
    }
    .into_iter()
    .for_each(|dop| {
        collect_single_dop(cda_database::datatypes::DataOperation(dop), all_dops, seen);
    });

    if matches!(param_type, ParamType::TableKey) {
        let tk = param.0.specific_data_as_table_key();

        tk.and_then(|tk| tk.table_key_reference_as_table_dop())
            .into_iter()
            .for_each(|table_dop| {
                table_dop.key_dop().into_iter().for_each(|kd| {
                    collect_single_dop(cda_database::datatypes::DataOperation(kd), all_dops, seen);
                });

                table_dop
                    .rows()
                    .into_iter()
                    .flat_map(|rows| rows.iter())
                    .for_each(|row| collect_table_row!(row));
            });

        tk.and_then(|tk| tk.table_key_reference_as_table_row())
            .into_iter()
            .for_each(|row| collect_table_row!(row));
    }

    if matches!(param_type, ParamType::TableEntry) {
        param
            .0
            .specific_data_as_table_entry()
            .and_then(|te| te.table_row())
            .into_iter()
            .for_each(|row| collect_table_row!(row));
    }

    if matches!(param_type, ParamType::TableStruct) {
        param
            .0
            .specific_data_as_table_struct()
            .and_then(|ts| ts.table_key())
            .into_iter()
            .for_each(|tk_param| collect_dop_from_param(&Parameter(tk_param), all_dops, seen));
    }
}

/// Collect all DOPs from a single `DiagLayer` (own services + single ECU jobs + `ComParamSubSet`)
fn collect_dops_from_layer<'a>(
    layer: &DiagLayer<'a>,
    all_dops: &mut Vec<DopInfo<'a>>,
    seen: &mut std::collections::HashSet<String>,
) {
    layer
        .diag_services()
        .into_iter()
        .flat_map(|s| s.iter())
        .for_each(|svc| collect_dops_from_service(&DiagService(svc), all_dops, seen));

    layer
        .single_ecu_jobs()
        .into_iter()
        .flat_map(|j| j.iter())
        .for_each(|job| {
            [
                job.input_params(),
                job.output_params(),
                job.neg_output_params(),
            ]
            .into_iter()
            .flatten()
            .flat_map(|params| params.iter())
            .filter_map(|jp| jp.dop_base())
            .for_each(|dop| {
                collect_single_dop(cda_database::datatypes::DataOperation(dop), all_dops, seen);
            });
        });

    layer
        .com_param_refs()
        .into_iter()
        .flat_map(|refs| refs.iter())
        .filter_map(|cpr| cpr.prot_stack())
        .flat_map(|ps| {
            ps.comparam_subset_refs()
                .into_iter()
                .flat_map(|subsets| subsets.iter())
        })
        .for_each(|subset| {
            subset
                .data_object_props()
                .into_iter()
                .flat_map(|dops| dops.iter())
                .for_each(|dop| {
                    collect_single_dop(cda_database::datatypes::DataOperation(dop), all_dops, seen);
                });
        });
}

/// Collect DOPs from a service's request and response params
fn collect_dops_from_service<'a>(
    ds: &DiagService<'a>,
    all_dops: &mut Vec<DopInfo<'a>>,
    seen: &mut std::collections::HashSet<String>,
) {
    macro_rules! collect_params {
        ($params:expr) => {
            $params
                .iter()
                .for_each(|param| collect_dop_from_param(&Parameter(param), all_dops, seen))
        };
    }

    ds.request()
        .and_then(|r| r.params())
        .into_iter()
        .for_each(|params| collect_params!(params));

    ds.pos_responses()
        .into_iter()
        .flat_map(|r| r.iter())
        .filter_map(|resp| resp.params())
        .for_each(|params| collect_params!(params));

    ds.neg_responses()
        .into_iter()
        .flat_map(|r| r.iter())
        .filter_map(|resp| resp.params())
        .for_each(|params| collect_params!(params));
}

/// Build overview table showing semantic categories and their counts
fn build_dops_overview_table(
    categories: &[(DopCategory, &[DopInfo<'_>])],
) -> Vec<DetailSectionData> {
    let header = DetailRow {
        cells: vec!["Category".to_owned(), "Count".to_owned()],
        cell_types: vec![CellType::Text, CellType::NumericValue],
        indent: 0,
        ..Default::default()
    };

    let rows: Vec<DetailRow> = categories
        .iter()
        .map(|(cat, dops)| DetailRow {
            cells: vec![cat.label().to_owned(), dops.len().to_string()],
            cell_types: vec![CellType::Text, CellType::NumericValue],
            cell_jump_targets: vec![None; 2],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        })
        .collect();

    vec![DetailSectionData {
        title: "Overview".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Overview,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(70),
                ColumnConstraint::Percentage(30),
            ],
            use_row_selection: true,
        },
    }]
}

/// Build overview table with only SHORT-NAME column.
/// Used for `Structures`, `StaticFields`, `DynamicLengthFields`, `EndOfPduFields`, `MuxDops`.
fn build_short_name_only_overview(dops: &[DopInfo<'_>]) -> Vec<DetailSectionData> {
    let header = DetailRow {
        cells: vec!["SHORT-NAME".to_owned()],
        cell_types: vec![CellType::Text],
        indent: 0,
        ..Default::default()
    };

    let rows: Vec<DetailRow> = dops
        .iter()
        .map(|dop_info| DetailRow {
            cells: vec![dop_info.name.clone()],
            cell_types: vec![CellType::Text],
            cell_jump_targets: vec![None; 1],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        })
        .collect();

    vec![DetailSectionData {
        title: "Overview".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Overview,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![ColumnConstraint::Percentage(100)],
            use_row_selection: true,
        },
    }]
}

// ── SDG helpers ──────────────────────────────────────────────────────────────

/// Convenience: build a key/value detail row.
fn kv_row(key: &str, value: String, value_type: CellType, indent: usize) -> DetailRow {
    DetailRow::normal(
        vec![key.to_owned(), value],
        vec![CellType::Text, value_type],
        indent,
    )
}

/// Build overview table for a semantic DOP category
fn build_category_overview_table(dops: &[DopInfo<'_>]) -> Vec<DetailSectionData> {
    let header = DetailRow {
        cells: vec![
            "SHORT-NAME".to_owned(),
            "CATEGORY".to_owned(),
            "Internal".to_owned(),
            "Physical".to_owned(),
            "DESC ID".to_owned(),
        ],
        cell_types: vec![
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
        ],
        indent: 0,
        ..Default::default()
    };

    let rows: Vec<DetailRow> = dops
        .iter()
        .map(|dop_info| DetailRow {
            cells: vec![
                dop_info.name.clone(),
                dop_info.category.as_deref().unwrap_or("").to_owned(),
                dop_info.internal_unit.as_deref().unwrap_or("").to_owned(),
                dop_info.phys_unit.as_deref().unwrap_or("").to_owned(),
                dop_info.desc_id.as_deref().unwrap_or("").to_owned(),
            ],
            cell_types: vec![
                CellType::Text,
                CellType::Text,
                CellType::Text,
                CellType::Text,
                CellType::Text,
            ],
            cell_jump_targets: vec![None; 5],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        })
        .collect();

    vec![DetailSectionData {
        title: "Overview".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Overview,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(25),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(15),
            ],
            use_row_selection: true,
        },
    }]
}

/// Helper to push a "Types" tab section from accumulated rows
fn push_types_section(types_rows: Vec<DetailRow>, sections: &mut Vec<DetailSectionData>) {
    if !types_rows.is_empty() {
        let header = DetailRow {
            cells: vec!["Property".to_owned(), "Value".to_owned()],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            ..Default::default()
        };
        sections.insert(
            0,
            DetailSectionData {
                title: "Types".to_owned(),
                render_as_header: false,
                section_type: DetailSectionType::Custom,
                content: DetailContent::Table {
                    header,
                    rows: types_rows,
                    constraints: vec![
                        ColumnConstraint::Percentage(40),
                        ColumnConstraint::Percentage(60),
                    ],
                    use_row_selection: true,
                },
            },
        );
    }
}

/// Build detail sections for a single DOP with full type-specific information
fn build_dop_detail_sections(dop_info: &DopInfo<'_>) -> Vec<DetailSectionData> {
    use cda_database::datatypes::DataOperationVariant;

    let mut sections = Vec::new();

    let parsed_name = parse_dop_name(&dop_info.name);

    let mut types_rows = Vec::new();
    types_rows.push(DetailRow {
        cells: vec!["Short Name".to_owned(), dop_info.name.clone()],
        cell_types: vec![CellType::Text, CellType::Text],
        cell_jump_targets: vec![None; 2],
        indent: 0,
        row_type: DetailRowType::Normal,
        metadata: None,
    });
    types_rows.push(DetailRow {
        cells: vec!["DOP Variant".to_owned(), dop_info.dop_type.clone()],
        cell_types: vec![CellType::Text, CellType::Text],
        cell_jump_targets: vec![None; 2],
        indent: 0,
        row_type: DetailRowType::Normal,
        metadata: None,
    });

    if let Some(ref compu) = parsed_name.compu_category {
        types_rows.push(DetailRow {
            cells: vec!["Compu Category (from name)".to_owned(), compu.clone()],
            cell_types: vec![CellType::Text, CellType::Text],
            cell_jump_targets: vec![None; 2],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        });
    }
    if let Some(ref unit) = parsed_name.unit {
        types_rows.push(DetailRow {
            cells: vec!["Unit (from name)".to_owned(), unit.clone()],
            cell_types: vec![CellType::Text, CellType::Text],
            cell_jump_targets: vec![None; 2],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        });
    }

    if let Ok(variant) = dop_info.dop.variant() {
        match variant {
            DataOperationVariant::Normal(normal_dop) => {
                normal_dops::build_normal_dop_tabs(
                    &normal_dop,
                    &parsed_name,
                    &mut types_rows,
                    &mut sections,
                );
            }
            DataOperationVariant::Structure(structure) => {
                structures::build_structure_dop_tabs(&structure, &mut types_rows, &mut sections);
            }
            DataOperationVariant::StaticField(static_field) => {
                static_fields::build_static_field_dop_tabs(
                    &static_field,
                    &mut types_rows,
                    &mut sections,
                );
            }
            DataOperationVariant::EndOfPdu(eof_field) => {
                end_of_pdu::build_end_of_pdu_dop_tabs(&eof_field, &mut types_rows, &mut sections);
            }
            DataOperationVariant::DynamicLengthField(dyn_field) => {
                dynamic_length_fields::build_dynamic_length_field_dop_tabs(
                    &dyn_field,
                    &mut types_rows,
                    &mut sections,
                );
            }
            DataOperationVariant::EnvDataDesc(env_desc) => {
                env_data::build_env_data_desc_dop_tabs(&env_desc, &mut types_rows, &mut sections);
            }
            DataOperationVariant::EnvData(env_data_var) => {
                env_data::build_env_data_dop_tabs(&env_data_var, &mut types_rows, &mut sections);
            }
            DataOperationVariant::Mux(mux_dop) => {
                mux_dops::build_mux_dop_tabs(&mux_dop, &mut types_rows, &mut sections);
            }
            DataOperationVariant::Dtc(dtc_dop) => {
                dtc_dops::build_dtc_dop_tabs(&dtc_dop, &mut types_rows, &mut sections);
            }
        }
    } else {
        push_types_section(types_rows, &mut sections);
    }

    sections
}
