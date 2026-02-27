// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use std::collections::HashMap;

use cda_database::datatypes::{Dtc, EcuDb};

use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ColumnConstraint, DetailContent, DetailRow, DetailSectionData, DetailSectionType,
        NodeType, SectionType,
    },
};

/// Format a trouble code for display, preferring the display code over hex.
fn format_trouble_code(dtc: &Dtc<'_>) -> String {
    dtc.display_trouble_code()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("0x{:06X}", dtc.trouble_code()))
}

/// Build the overview table shown on the DTCs section header.
fn build_overview_section(dtcs: &[&Dtc<'_>]) -> DetailSectionData {
    let rows: Vec<DetailRow> = dtcs
        .iter()
        .map(|dtc| {
            let short_name = dtc.short_name().unwrap_or("?").to_owned();
            let code = format_trouble_code(dtc);
            let description = dtc
                .text()
                .and_then(|t| t.value())
                .unwrap_or("")
                .to_owned();
            DetailRow::normal(
                vec![short_name, code, description],
                vec![CellType::Text, CellType::Text, CellType::Text],
                0,
            )
        })
        .collect();

    let header = DetailRow::header(
        vec![
            "Short Name".to_owned(),
            "Code".to_owned(),
            "Description".to_owned(),
        ],
        vec![CellType::Text, CellType::Text, CellType::Text],
    );

    DetailSectionData::new(
        "DTCs Overview".to_owned(),
        DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(30),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(50),
            ],
            use_row_selection: true,
        },
        false,
    )
    .with_type(DetailSectionType::Overview)
}

/// Build the detail sections shown when a single DTC node is selected.
fn build_dtc_detail_sections(dtc: &Dtc<'_>) -> Vec<DetailSectionData> {
    let short_name = dtc.short_name().unwrap_or("?");
    let code = format_trouble_code(dtc);

    let header_section = DetailSectionData {
        title: format!("DTC - {} - {}", code, short_name),
        render_as_header: true,
        content: DetailContent::PlainText(vec![]),
        section_type: DetailSectionType::Header,
    };

    let overview_section = build_dtc_overview(dtc, short_name);
    let sdg_section = build_sdg_section(dtc);

    vec![header_section, overview_section, sdg_section]
}

/// Build the key/value overview table for a single DTC.
fn build_dtc_overview(dtc: &Dtc<'_>, short_name: &str) -> DetailSectionData {
    let kv = |key: &str, val: String| {
        DetailRow::normal(
            vec![key.to_owned(), val],
            vec![CellType::Text, CellType::Text],
            0,
        )
    };

    let trouble_code = dtc.trouble_code();
    let display_code = dtc.display_trouble_code().unwrap_or("");

    let rows: Vec<DetailRow> = [
        Some(kv("Short Name", short_name.to_owned())),
        Some(kv("Trouble Code", format!("0x{:06X}", trouble_code))),
        (!display_code.is_empty())
            .then(|| kv("Display Code", display_code.to_owned())),
        dtc.text()
            .and_then(|t| t.value())
            .map(|v| kv("Description", v.to_owned())),
        dtc.level().map(|l| kv("Level", l.to_string())),
        Some(kv(
            "Temporary",
            if dtc.is_temporary() { "Yes" } else { "No" }.to_owned(),
        )),
    ]
    .into_iter()
    .flatten()
    .collect();

    let header = DetailRow::header(
        vec!["Property".to_owned(), "Value".to_owned()],
        vec![CellType::Text, CellType::Text],
    );

    DetailSectionData::new(
        "Overview".to_owned(),
        DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(30),
                ColumnConstraint::Percentage(70),
            ],
            use_row_selection: true,
        },
        false,
    )
    .with_type(DetailSectionType::Overview)
}

/// Build the SDGs section for a single DTC.
fn build_sdg_section(dtc: &Dtc<'_>) -> DetailSectionData {
    let sdg_list: Vec<_> = dtc
        .sdgs()
        .and_then(|sdgs| sdgs.sdgs())
        .into_iter()
        .flat_map(|list| list.iter())
        .collect();

    if sdg_list.is_empty() {
        return DetailSectionData::new(
            "SDGs".to_owned(),
            DetailContent::PlainText(vec!["(No SDGs available)".to_owned()]),
            false,
        )
        .with_type(DetailSectionType::Custom);
    }

    // Build one sub-table per SDG so sorting stays within each group
    let subsections: Vec<DetailSectionData> = sdg_list
        .iter()
        .flat_map(|sdg| {
            let caption = sdg.caption_sn().unwrap_or("");
            let si = sdg.si().unwrap_or("-");

            let sd_rows: Vec<DetailRow> = sdg
                .sds()
                .into_iter()
                .flat_map(|sds| sds.iter())
                .filter_map(|entry| entry.sd_or_sdg_as_sd())
                .map(|sd| {
                    DetailRow::normal(
                        vec![
                            sd.value().unwrap_or("-").to_owned(),
                            sd.si().unwrap_or("-").to_owned(),
                            sd.ti().unwrap_or("-").to_owned(),
                        ],
                        vec![CellType::Text, CellType::Text, CellType::Text],
                        0,
                    )
                })
                .collect();

            let label = DetailSectionData {
                title: String::new(),
                render_as_header: false,
                section_type: DetailSectionType::Custom,
                content: DetailContent::PlainText(vec![format!("SDG: {caption}  (SI: {si})")]),
            };

            let table = DetailSectionData {
                title: String::new(),
                render_as_header: false,
                section_type: DetailSectionType::Custom,
                content: DetailContent::Table {
                    header: DetailRow::header(
                        vec![
                            "Value".to_owned(),
                            "SI".to_owned(),
                            "TI".to_owned(),
                        ],
                        vec![CellType::Text, CellType::Text, CellType::Text],
                    ),
                    rows: sd_rows,
                    constraints: vec![
                        ColumnConstraint::Percentage(50),
                        ColumnConstraint::Percentage(25),
                        ColumnConstraint::Percentage(25),
                    ],
                    use_row_selection: true,
                },
            };

            vec![label, table]
        })
        .collect();

    DetailSectionData::new(
        "SDGs".to_owned(),
        DetailContent::Composite(subsections),
        false,
    )
    .with_type(DetailSectionType::Custom)
}

/// Deduplicate and sort DTCs, keeping the first occurrence of each trouble code.
fn unique_sorted_dtcs<'a>(dtcs: impl Iterator<Item = Dtc<'a>>) -> Vec<Dtc<'a>> {
    let mut seen = HashMap::new();
    for dtc in dtcs {
        seen.entry(dtc.trouble_code()).or_insert(dtc);
    }
    let mut out: Vec<_> = seen.into_values().collect();
    out.sort_by_key(|d| d.trouble_code());
    out
}

/// Add all DTCs to the tree.
pub fn add_dtcs(b: &mut TreeBuilder, ecu: &EcuDb<'_>) {
    let dtcs = match ecu.dtcs() {
        Some(d) if !d.is_empty() => d,
        _ => return,
    };

    let unique = unique_sorted_dtcs(dtcs.iter());
    let refs: Vec<&Dtc<'_>> = unique.iter().collect();

    b.push_section_header(
        format!("DTCs ({})", unique.len()),
        false,
        true,
        vec![build_overview_section(&refs)],
        SectionType::DTCs,
    );

    for dtc in &unique {
        let short_name = dtc.short_name().unwrap_or("?");
        let display_name = format!("{} - {}", format_trouble_code(dtc), short_name);

        b.push_details_structured(
            1,
            display_name,
            false,
            false,
            build_dtc_detail_sections(dtc),
            NodeType::Default,
        );
    }
}
