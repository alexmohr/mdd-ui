/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::DataOperationVariant;

use super::{DopInfo, kv_row, push_types_section};
use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ColumnConstraint, DetailCell, DetailContent, DetailRow, DetailSectionData,
        DetailSectionType, NodeType,
    },
};

/// Build sections for the DTC-DOPS category node.
/// Overview with only SHORT-NAME column.
pub(super) fn build_dtc_dops_category_sections(dops: &[DopInfo<'_>]) -> Vec<DetailSectionData> {
    let header = DetailRow::header(vec![DetailCell::text("SHORT-NAME")]);

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
            DetailRow::normal(vec![DetailCell::text(name)], 0)
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
            $rows.push(DetailRow::header(vec![DetailCell::text($caption.clone()), DetailCell::text("")]));
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
                vec![DetailCell::text("Short Name"), DetailCell::text(short_name)],
                0,
            ),
            DetailRow::normal(
                vec![
                    DetailCell::text("Trouble Code (numeric)"),
                    DetailCell::text(format!("0x{:06X} ({})", dtc.trouble_code(), dtc.trouble_code())),
                ],
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
                header: DetailRow::header(vec![DetailCell::text("Property"), DetailCell::text("Value")]),
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
        types_rows.push(DetailRow::normal(
            vec![DetailCell::text("Diag Coded Type"), DetailCell::text(format!("{:?}", coded_type.base_datatype()))],
            0,
        ));
    }

    if let Some(dtcs) = dtc_dop.dtcs() {
        types_rows.push(DetailRow::normal(
            vec![DetailCell::text("DTC Count"), DetailCell::new(dtcs.len().to_string(), CellType::NumericValue)],
            0,
        ));
    }

    push_types_section(std::mem::take(types_rows), sections);

    if let Some(dtcs) = dtc_dop.dtcs() {
        let compu_category_label = dtc_dop
            .compu_method()
            .map(|cm| format!("{:?}", cm.category()));

        let dtcs_header = DetailRow::header(vec![
            DetailCell::text("Short Name"),
            DetailCell::text("Trouble Code"),
            DetailCell::text("Text"),
        ]);

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

                DetailRow::normal(
                    vec![DetailCell::text(short_name), DetailCell::text(code_str), DetailCell::text(text)],
                    0,
                )
            })
            .collect();

        let title = compu_category_label
            .map_or_else(|| "DTCs".to_owned(), |cat| format!("DTCs (Compu: {cat})"));

        sections.push(DetailSectionData {
            title,
            render_as_header: false,
            section_type: DetailSectionType::Overview,
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
        });
    }
}
