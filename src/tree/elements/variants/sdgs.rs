/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::DiagLayer;

use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ColumnConstraint, DetailContent, DetailRow, DetailSectionData, DetailSectionType,
        NodeType,
    },
};

/// Add SDGs (Service Data Groups) section to the tree
pub fn add_sdgs(b: &mut TreeBuilder, layer: &DiagLayer<'_>, depth: usize) {
    // Get SDGs from the layer
    let Some(sdgs) = layer.sdgs() else {
        return;
    };

    let Some(sdg_list) = sdgs.sdgs() else {
        return;
    };

    if sdg_list.is_empty() {
        return;
    }

    // Extract all SDG data including SD elements
    let mut sdg_data_list = Vec::new();
    for sdg in sdg_list.iter() {
        let Some(caption) = sdg.caption_sn() else {
            continue;
        };
        let si = sdg.si().unwrap_or("-");

        // Extract SD elements from this SDG
        let sd_elements: Vec<_> = sdg
            .sds()
            .into_iter()
            .flat_map(|sds| sds.iter().enumerate())
            .filter_map(|(index, sd_or_sdg)| {
                let sd = sd_or_sdg.sd_or_sdg_as_sd()?;
                let short_name = sd
                    .si()
                    .map(|s| s.to_owned())
                    .unwrap_or_else(|| format!("SD #{}", index + 1));
                Some(SdElement {
                    short_name,
                    value: sd.value().unwrap_or("-").to_owned(),
                    si: sd.si().unwrap_or("-").to_owned(),
                    ti: sd.ti().unwrap_or("-").to_owned(),
                    depth: 0,
                })
            })
            .collect();

        sdg_data_list.push(SdgData {
            caption: caption.to_owned(),
            si: si.to_owned(),
            sd_elements,
        });
    }

    if sdg_data_list.is_empty() {
        return;
    }

    // Build the SDGs table section
    let sdgs_table = build_sdgs_table_section(&sdg_data_list);

    b.push_details_structured(
        depth,
        format!("SDGs ({})", sdg_data_list.len()),
        false,
        true,
        vec![sdgs_table],
        NodeType::SectionHeader,
    );

    // Add each SDG as a child node with detail
    for sdg_data in &sdg_data_list {
        let detail_sections = build_sdg_detail_sections(sdg_data);

        b.push_details_structured(
            depth + 1,
            sdg_data.caption.clone(),
            false,
            false,
            detail_sections,
            NodeType::SDG,
        );
    }
}

/// Build a table section showing all SDGs in the list view
fn build_sdgs_table_section(sdg_data_list: &[SdgData]) -> DetailSectionData {
    let header = DetailRow::header(
        vec!["Short Name".to_owned(), "SI".to_owned()],
        vec![CellType::Text, CellType::Text],
    );

    let rows: Vec<_> = sdg_data_list
        .iter()
        .map(|sdg_data| {
            DetailRow::normal(
                vec![sdg_data.caption.clone(), sdg_data.si.clone()],
                vec![CellType::Text, CellType::Text],
                0,
            )
        })
        .collect();

    DetailSectionData::new(
        format!("SDGs ({})", sdg_data_list.len()),
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
    .with_type(DetailSectionType::Custom)
}

/// Build detailed sections for a single SDG
fn build_sdg_detail_sections(sdg_data: &SdgData) -> Vec<DetailSectionData> {
    let mut sections = Vec::new();

    // Add header section
    sections.push(DetailSectionData {
        title: format!("SDG - {}", sdg_data.caption),
        render_as_header: true,
        section_type: DetailSectionType::Header,
        content: DetailContent::PlainText(vec![]),
    });

    // Overview section with SDG metadata
    sections.push(build_sdg_overview_section(sdg_data));

    // SD elements table (will be shown when SD elements are extracted)
    if !sdg_data.sd_elements.is_empty() {
        sections.push(build_sd_elements_table_section(&sdg_data.sd_elements));
    }

    sections
}

/// Build overview section for SDG
fn build_sdg_overview_section(sdg_data: &SdgData) -> DetailSectionData {
    let header = DetailRow::header(
        vec!["Property".to_owned(), "Value".to_owned()],
        vec![CellType::Text, CellType::Text],
    );

    let rows = vec![
        DetailRow::normal(
            vec!["Short Name".to_owned(), sdg_data.caption.clone()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
        DetailRow::normal(
            vec!["SI".to_owned(), sdg_data.si.clone()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
    ];

    DetailSectionData::new(
        "Overview".to_owned(),
        DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(30),
                ColumnConstraint::Percentage(70),
            ],
            use_row_selection: false,
        },
        false,
    )
    .with_type(DetailSectionType::Overview)
}

/// Build table section showing SD elements with Name | Value columns
fn build_sd_elements_table_section(sd_elements: &[SdElement]) -> DetailSectionData {
    let header = DetailRow::header(
        vec!["Name".to_owned(), "Value".to_owned()],
        vec![CellType::Text, CellType::Text],
    );

    let rows: Vec<_> = sd_elements
        .iter()
        .map(|sd| {
            DetailRow::normal(
                vec![sd.short_name.clone(), sd.value.clone()],
                vec![CellType::Text, CellType::Text],
                sd.depth,
            )
        })
        .collect();

    DetailSectionData::new(
        "Service Data (SD)".to_owned(),
        DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(40),
                ColumnConstraint::Percentage(60),
            ],
            use_row_selection: true,
        },
        false,
    )
    .with_type(DetailSectionType::Custom)
}

// Data structures
struct SdgData {
    caption: String,
    si: String,
    sd_elements: Vec<SdElement>,
}

struct SdElement {
    short_name: String,
    value: String,
    #[allow(dead_code)]
    si: String,
    #[allow(dead_code)]
    ti: String,
    depth: usize,
}
