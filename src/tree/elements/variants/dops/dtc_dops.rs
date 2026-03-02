/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::DataOperationVariant;

use super::{DopInfo, kv_row, push_types_section};
use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ColumnConstraint, DetailContent, DetailRow, DetailRowType, DetailSectionData,
        DetailSectionType, NodeType,
    },
};

/// Build sections for the DTC-DOPS category node.
/// Overview with only SHORT-NAME column.
pub(super) fn build_dtc_dops_category_sections(dops: &[DopInfo<'_>]) -> Vec<DetailSectionData> {
    let header = DetailRow {
        cells: vec!["SHORT-NAME".to_owned()],
        cell_types: vec![CellType::Text],
        indent: 0,
        ..Default::default()
    };

    let rows: Vec<DetailRow> = dops
        .iter()
        .map(|dop_info| {
            let parsed = super::parse_dop_name(&dop_info.name);
            let display = parsed.display_name();
            let name = if display.is_empty() {
                dop_info.name.clone()
            } else {
                display
            };
            DetailRow {
                cells: vec![name],
                cell_types: vec![CellType::Text],
                cell_jump_targets: vec![None; 1],
                indent: 0,
                row_type: DetailRowType::Normal,
                metadata: None,
            }
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

/// Pick a label for a raw SD entry: prefer si, then ti, then a numbered fallback.
fn sd_entry_label(
    si: &str,
    ti: &str,
    caption: &str,
    unnamed_idx: &mut usize,
    unnamed_count: usize,
) -> String {
    if !si.is_empty() {
        si.to_owned()
    } else if !ti.is_empty() {
        ti.to_owned()
    } else if unnamed_count > 1 {
        *unnamed_idx = unnamed_idx.saturating_add(1);
        format!("{caption} [{unnamed_idx}]")
    } else {
        caption.to_owned()
    }
}

/// Helper: count unnamed SD entries and detect named/nested entries in an SDG's
/// `sds()` list. Returns `(has_named_or_nested, unnamed_count)`.
macro_rules! sdg_entry_stats {
    ($sds:expr) => {{
        let has_named_or_nested = $sds.iter().any(|e| {
            e.sd_or_sdg_as_sd()
                .map(|sd| !sd.si().unwrap_or("").is_empty() || !sd.ti().unwrap_or("").is_empty())
                .unwrap_or(true) // nested SDG => treat as named
        });
        let unnamed_count = $sds
            .iter()
            .filter(|e| {
                e.sd_or_sdg_as_sd()
                    .map(|sd| sd.si().unwrap_or("").is_empty() && sd.ti().unwrap_or("").is_empty())
                    .unwrap_or(false)
            })
            .count();
        (has_named_or_nested, unnamed_count)
    }};
}

/// Helper: emit an SDG header row + optional SI row when the group has named
/// or nested children.  Returns the indent level for child rows.
macro_rules! emit_sdg_header {
    ($rows:expr, $sdg:expr, $caption:expr, $has_named:expr, $base_indent:expr) => {{
        if $has_named {
            $rows.push(DetailRow {
                cells: vec![$caption.clone(), String::new()],
                cell_types: vec![CellType::Text, CellType::Text],
                cell_jump_targets: vec![None; 2],
                indent: $base_indent,
                row_type: DetailRowType::Header,
                metadata: None,
            });
            if let Some(si) = $sdg.si() {
                $rows.push(kv_row(
                    "SI",
                    si.to_owned(),
                    CellType::Text,
                    $base_indent.saturating_add(1),
                ));
            }
            $base_indent.saturating_add(1)
        } else {
            $base_indent
        }
    }};
}

/// Helper: emit SD value rows from an SDG's `sds()` list.
macro_rules! emit_sd_rows {
    ($rows:expr, $sds:expr, $caption:expr, $unnamed_count:expr, $indent:expr) => {{
        let mut unnamed_idx = 0usize;
        for entry in $sds.iter() {
            if let Some(sd) = entry.sd_or_sdg_as_sd() {
                let label = sd_entry_label(
                    sd.si().unwrap_or(""),
                    sd.ti().unwrap_or(""),
                    &$caption,
                    &mut unnamed_idx,
                    $unnamed_count,
                );
                let value = sd.value().unwrap_or("").to_owned();
                $rows.push(kv_row(&label, value, CellType::Text, $indent));
            }
        }
    }};
}

/// Append flattened SDG rows (up to two nesting levels) into `rows`.
macro_rules! append_sdg_rows {
    ($rows:expr, $dtc:expr) => {{
        let sdgs = $dtc.sdgs().and_then(|s| s.sdgs());
        if let Some(groups) = sdgs {
            for sdg in groups.iter() {
                let caption = sdg.caption_sn().unwrap_or("SDG").to_owned();
                let Some(sds) = sdg.sds() else { continue };

                let (has_named, unnamed_count) = sdg_entry_stats!(sds);
                let indent = emit_sdg_header!($rows, sdg, caption, has_named, 0usize);
                emit_sd_rows!($rows, sds, caption, unnamed_count, indent);

                for entry in sds.iter() {
                    if let Some(nested) = entry.sd_or_sdg_as_sdg() {
                        let n_caption = nested.caption_sn().unwrap_or("SDG").to_owned();
                        if let Some(n_sds) = nested.sds() {
                            let (n_has_named, n_unnamed_count) = sdg_entry_stats!(n_sds);
                            let n_indent =
                                emit_sdg_header!($rows, nested, n_caption, n_has_named, indent);
                            emit_sd_rows!($rows, n_sds, n_caption, n_unnamed_count, n_indent);
                        }
                    }
                }
            }
        }
    }};
}

/// Add DTC child nodes under an individual DTC-DOP tree node
pub(super) fn add_dtc_dop_children(b: &mut TreeBuilder, dop_info: &DopInfo<'_>, depth: usize) {
    let Ok(DataOperationVariant::Dtc(dtc_dop)) = dop_info.dop.variant() else {
        return;
    };
    let Some(dtcs) = dtc_dop.dtcs() else {
        return;
    };

    for dtc in dtcs {
        let short_name = dtc.short_name().unwrap_or("?").to_owned();
        let code_str = dtc
            .display_trouble_code()
            .filter(|s| !s.is_empty())
            .map_or_else(|| format!("0x{:06X}", dtc.trouble_code()), str::to_owned);
        let text = dtc.text().and_then(|t| t.value()).unwrap_or("").to_owned();

        let display_name = format!("{short_name} - {code_str}");

        let mut rows: Vec<DetailRow> = vec![
            DetailRow::normal(
                vec!["Short Name".to_owned(), short_name],
                vec![CellType::Text, CellType::Text],
                0,
            ),
            DetailRow::normal(
                vec![
                    "Trouble Code (numeric)".to_owned(),
                    format!("0x{:06X} ({})", dtc.trouble_code(), dtc.trouble_code()),
                ],
                vec![CellType::Text, CellType::Text],
                0,
            ),
        ];

        let optional_rows: Vec<DetailRow> = [
            dtc.display_trouble_code()
                .map(|dc| kv_row("Display Trouble Code", dc.to_owned(), CellType::Text, 0)),
            (!text.is_empty()).then(|| kv_row("Text", text, CellType::Text, 0)),
            dtc.text()
                .and_then(|t| t.ti())
                .map(|ti| kv_row("Text ID (ti)", ti.to_owned(), CellType::Text, 0)),
            dtc.level()
                .map(|l| kv_row("Level (Severity)", l.to_string(), CellType::NumericValue, 0)),
            Some(kv_row(
                "Is Temporary",
                dtc.is_temporary().to_string(),
                CellType::Text,
                0,
            )),
        ]
        .into_iter()
        .flatten()
        .collect();

        rows.extend(optional_rows);

        append_sdg_rows!(rows, dtc);

        let detail = vec![DetailSectionData {
            title: "Overview".to_owned(),
            render_as_header: false,
            section_type: DetailSectionType::Overview,
            content: DetailContent::Table {
                header: DetailRow::header(
                    vec!["Property".to_owned(), "Value".to_owned()],
                    vec![CellType::Text, CellType::Text],
                ),
                rows,
                constraints: vec![
                    ColumnConstraint::Percentage(30),
                    ColumnConstraint::Percentage(70),
                ],
                use_row_selection: true,
            },
        }];

        b.push_details_structured(depth, display_name, false, false, detail, NodeType::Default);
    }
}

/// Build tabbed sections for DTCDOP: Summary (types) + DTCS table
pub(super) fn build_dtc_dop_tabs(
    dtc_dop: &cda_database::datatypes::DtcDop<'_>,
    types_rows: &mut Vec<DetailRow>,
    sections: &mut Vec<DetailSectionData>,
) {
    if let Ok(coded_type) = dtc_dop.diag_coded_type() {
        types_rows.push(DetailRow {
            cells: vec![
                "Diag Coded Type".to_owned(),
                format!("{:?}", coded_type.base_datatype()),
            ],
            cell_types: vec![CellType::Text, CellType::Text],
            cell_jump_targets: vec![None; 2],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        });
    }

    if let Some(dtcs) = dtc_dop.dtcs() {
        types_rows.push(DetailRow {
            cells: vec!["DTC Count".to_owned(), dtcs.len().to_string()],
            cell_types: vec![CellType::Text, CellType::NumericValue],
            cell_jump_targets: vec![None; 2],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        });
    }

    push_types_section(std::mem::take(types_rows), sections);

    if let Some(dtcs) = dtc_dop.dtcs() {
        let compu_category_label = dtc_dop
            .compu_method()
            .map(|cm| format!("Compu Category: {:?}", cm.category()));

        let dtcs_header = DetailRow {
            cells: vec![
                "Short Name".to_owned(),
                "Trouble Code".to_owned(),
                "Text".to_owned(),
            ],
            cell_types: vec![CellType::Text, CellType::Text, CellType::Text],
            indent: 0,
            ..Default::default()
        };

        let dtcs_rows: Vec<DetailRow> = dtcs
            .iter()
            .map(|dtc| {
                let short_name = dtc.short_name().unwrap_or("?").to_owned();
                let display_code = dtc.display_trouble_code().unwrap_or("");
                let code_str = if display_code.is_empty() {
                    format!("0x{:06X}", dtc.trouble_code())
                } else {
                    display_code.to_owned()
                };
                let text = dtc.text().and_then(|t| t.value()).unwrap_or("").to_owned();

                DetailRow {
                    cells: vec![short_name, code_str, text],
                    cell_types: vec![CellType::Text, CellType::Text, CellType::Text],
                    cell_jump_targets: vec![None; 3],
                    indent: 0,
                    row_type: DetailRowType::Normal,
                    metadata: None,
                }
            })
            .collect();

        let table_section = DetailSectionData {
            title: String::new(),
            render_as_header: false,
            section_type: DetailSectionType::Custom,
            content: DetailContent::Table {
                header: dtcs_header,
                rows: dtcs_rows,
                constraints: vec![
                    ColumnConstraint::Percentage(10),
                    ColumnConstraint::Percentage(10),
                    ColumnConstraint::Percentage(80),
                ],
                use_row_selection: true,
            },
        };

        let mut subsections = Vec::new();
        if let Some(label) = compu_category_label {
            subsections.push(DetailSectionData {
                title: String::new(),
                render_as_header: false,
                section_type: DetailSectionType::Custom,
                content: DetailContent::PlainText(vec![label]),
            });
        }
        subsections.push(table_section);

        sections.push(DetailSectionData {
            title: "DTCS".to_owned(),
            render_as_header: false,
            section_type: DetailSectionType::Overview,
            content: DetailContent::Composite(subsections),
        });
    }
}
