/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::{DiagLayer, DiagService, Variant};

use super::format_service_id;
use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellJumpTarget, CellJumpTargetType, CellType, ColumnConstraint, DetailCell, DetailContent,
        DetailRow, DetailSectionData, DetailSectionType,
    },
};

/// Collected services (with source layer name) and jobs (name + layer name) for a functional class.
type FcServicesAndJobs<'a> = (Vec<(DiagService<'a>, String)>, Vec<(String, String)>);

/// Check whether a `DiagComm`'s functional-class list includes the given name.
macro_rules! belongs_to_fc {
    ($dc:expr, $fc_name:expr) => {
        $dc.funct_class().is_some_and(|fcs| {
            fcs.iter()
                .any(|fc| fc.short_name().is_some_and(|n| n == $fc_name))
        })
    };
}

/// Add functional classes section from the diagnostic layer
/// This displays the FUNCT-CLASS definitions themselves and the services that belong to them
/// We collect services/jobs from ALL variants that have the same functional class
pub fn add_functional_classes<'a>(
    b: &mut TreeBuilder,
    layer: &DiagLayer<'_>,
    depth: usize,
    all_variants: Option<impl Iterator<Item = Variant<'a>> + 'a>,
) {
    // Collect all unique functional class names from this layer (base variant)
    let mut all_funct_class_names = std::collections::HashSet::new();

    // Get functional class definitions from the base layer
    if let Some(funct_classes) = layer.funct_classes() {
        for fc in funct_classes {
            if let Some(name) = fc.short_name() {
                all_funct_class_names.insert(name.to_string());
            }
        }
    }

    if all_funct_class_names.is_empty() {
        return;
    }

    // Convert to sorted vector for consistent display
    let mut funct_class_data: Vec<String> = all_funct_class_names.into_iter().collect();
    funct_class_data.sort();

    let count = funct_class_data.len();

    // Push header first (empty details — patched below with indices).
    let header_idx = b.next_index();
    b.push_service_list_header(
        depth,
        format!("Functional Classes ({count})"),
        false,
        true,
        vec![],
        crate::tree::ServiceListType::FunctionalClasses,
    );

    // Collect all services and jobs from ALL variants for each functional class
    // We'll do this per functional class below, searching across all variants
    let variants_vec: Vec<Variant<'_>> = all_variants
        .map(std::iter::Iterator::collect)
        .unwrap_or_default();

    // Push children, collecting (name → tree index).
    let mut node_indices = std::collections::HashMap::new();
    for name in &funct_class_data {
        // Collect services and jobs for this functional class
        let (mut all_services, mut all_job_info) = if variants_vec.is_empty() {
            // No variants provided, search only in the current layer
            collect_services_and_jobs_from_layer(name, layer)
        } else {
            // Variants provided, search across all of them
            collect_services_and_jobs_for_functional_class(name, &variants_vec)
        };

        // Sort services alphabetically by name
        all_services.sort_by_cached_key(|(ds, _)| {
            ds.diag_comm()
                .and_then(|dc| dc.short_name())
                .unwrap_or("")
                .to_lowercase()
        });

        // Sort jobs alphabetically by name
        all_job_info.sort_by_cached_key(|(job_name, _)| job_name.to_lowercase());

        // Build detailed view for this functional class
        let details = build_functional_class_detail(name, &all_services, &all_job_info);

        node_indices.insert(name.clone(), b.next_index());
        b.push_functional_class(depth.saturating_add(1), name.clone(), name.clone(), details);
    }

    // Build table with tree-node indices and patch header.
    let detail_section = build_functional_classes_table_section(&funct_class_data, &node_indices);
    b.set_detail_sections(header_idx, vec![detail_section]);
}

/// Build a table section for the Functional Classes header showing all class definitions
fn build_functional_classes_table_section(
    items: &[String],
    node_indices: &std::collections::HashMap<String, usize>,
) -> DetailSectionData {
    let header = DetailRow::header(vec![DetailCell::text("Short Name")]);

    let rows: Vec<_> = items
        .iter()
        .map(|name| {
            let jump = make_index_jump(name, node_indices);
            DetailRow::normal(
                vec![DetailCell::new(name.clone(), CellType::ParameterName).with_jump(jump)],
                0,
            )
        })
        .collect();

    DetailSectionData::new(
        format!("Functional Classes ({})", items.len()),
        DetailContent::Table {
            header,
            rows,
            constraints: vec![ColumnConstraint::Percentage(100)],
            use_row_selection: true,
        },
        false,
    )
    .with_type(DetailSectionType::FunctionalClass)
}

fn make_index_jump(
    short_name: &str,
    node_indices: &std::collections::HashMap<String, usize>,
) -> CellJumpTarget {
    let index = node_indices.get(short_name).copied().unwrap_or(usize::MAX);
    CellJumpTarget::new(CellJumpTargetType::TreeNodeByIndex {
        index,
        short_name: short_name.to_owned(),
    })
}

/// Collect services and jobs for a specific functional class from a single layer
/// This is used when no variants are provided (e.g., for functional groups or ECU shared data)
fn collect_services_and_jobs_from_layer<'a>(
    fc_name: &str,
    layer: &DiagLayer<'a>,
) -> FcServicesAndJobs<'a> {
    let layer_name = layer.short_name().unwrap_or("Unknown");

    let services: Vec<_> = layer
        .diag_services()
        .into_iter()
        .flatten()
        .filter_map(|service| {
            let service_wrap = DiagService(service);
            let dc = service_wrap.diag_comm()?;
            belongs_to_fc!(dc, fc_name).then_some((service_wrap, layer_name.to_string()))
        })
        .collect();

    let job_info: Vec<_> = layer
        .single_ecu_jobs()
        .into_iter()
        .flatten()
        .filter_map(|job| {
            let dc = job.diag_comm()?;
            let name = dc.short_name()?;
            belongs_to_fc!(dc, fc_name).then_some((name.to_string(), layer_name.to_string()))
        })
        .collect();

    (services, job_info)
}

/// Collect services and jobs for a specific functional class from ALL variants
fn collect_services_and_jobs_for_functional_class<'a>(
    fc_name: &str,
    all_variants: &[Variant<'a>],
) -> FcServicesAndJobs<'a> {
    let mut services = Vec::new();
    let mut job_info = Vec::new();
    let mut seen_services = std::collections::HashSet::new();
    let mut seen_jobs = std::collections::HashSet::new();

    for variant_wrap in all_variants {
        let variant_layer = match variant_wrap.diag_layer() {
            Some(layer) => DiagLayer(layer),
            None => continue,
        };

        let variant_name = variant_layer.short_name().unwrap_or("Unknown");

        if let Some(diag_services) = variant_layer.diag_services() {
            for service in diag_services {
                let service_wrap = DiagService(service);
                let Some(dc) = service_wrap.diag_comm() else {
                    continue;
                };
                let Some(short_name) = dc.short_name() else {
                    continue;
                };
                if belongs_to_fc!(dc, fc_name) && seen_services.insert(short_name.to_owned()) {
                    services.push((service_wrap, variant_name.to_string()));
                }
            }
        }

        if let Some(ecu_jobs) = variant_layer.single_ecu_jobs() {
            for job in ecu_jobs {
                let Some(job_dc) = job.diag_comm() else {
                    continue;
                };
                let Some(short_name) = job_dc.short_name() else {
                    continue;
                };
                if belongs_to_fc!(job_dc, fc_name) && seen_jobs.insert(short_name.to_owned()) {
                    job_info.push((short_name.to_string(), variant_name.to_string()));
                }
            }
        }
    }

    (services, job_info)
}

/// Build detailed view for a single functional class
/// Shows the services/jobs that belong to this functional class across all variants
fn build_service_row(service: &DiagService<'_>, layer_name: &str) -> Option<DetailRow> {
    let dc = service.diag_comm()?;
    let short_name = dc.short_name().unwrap_or("?").to_owned();
    let service_type = "Service".to_owned();

    let id_str = format_service_id(service);
    let sid_rq = if id_str.is_empty() {
        "-".to_owned()
    } else {
        id_str
    };

    let semantic = dc.semantic().unwrap_or("-").to_owned();
    let addressing = format!("{:?}", service.addressing());

    Some(DetailRow::normal(
        vec![
            DetailCell::new(short_name.clone(), CellType::ParameterName).with_jump(
                CellJumpTarget::new(CellJumpTargetType::TreeNodeByIndex {
                    index: usize::MAX,
                    short_name,
                }),
            ),
            DetailCell::text(service_type),
            DetailCell::text(sid_rq),
            DetailCell::text(semantic),
            DetailCell::text(addressing),
            DetailCell::new(layer_name, CellType::ParameterName).with_jump(CellJumpTarget::new(
                CellJumpTargetType::TreeNodeByIndex {
                    index: usize::MAX,
                    short_name: layer_name.to_owned(),
                },
            )),
        ],
        0,
    ))
}

fn build_job_row(job_name: &str, layer_name: &str) -> DetailRow {
    DetailRow::normal(
        vec![
            DetailCell::new(job_name, CellType::ParameterName).with_jump(CellJumpTarget::new(
                CellJumpTargetType::TreeNodeByIndex {
                    index: usize::MAX,
                    short_name: job_name.to_owned(),
                },
            )),
            DetailCell::text("Job"),
            DetailCell::text("-"),
            DetailCell::text("-"),
            DetailCell::text("-"),
            DetailCell::new(layer_name, CellType::ParameterName).with_jump(CellJumpTarget::new(
                CellJumpTargetType::TreeNodeByIndex {
                    index: usize::MAX,
                    short_name: layer_name.to_owned(),
                },
            )),
        ],
        0,
    )
}

fn build_functional_class_detail(
    fc_name: &str,
    services: &[(DiagService<'_>, String)],
    all_job_info: &[(String, String)], // (job_name, layer_name)
) -> Vec<DetailSectionData> {
    let mut sections = Vec::new();

    // Add header section with functional class name
    sections.push(DetailSectionData {
        title: format!("Functional Class: {fc_name}"),
        render_as_header: true,
        content: DetailContent::PlainText(vec![]),
        section_type: DetailSectionType::Header,
        byte_pattern_rows: None,
    });

    // Build services table
    let header = DetailRow::header(vec![
        DetailCell::text("Short Name"),
        DetailCell::text("Type"),
        DetailCell::text("SID_RQ"),
        DetailCell::text("Semantic"),
        DetailCell::text("Addressing"),
        DetailCell::text("Layer"),
    ]);

    let mut rows = Vec::new();

    // Add each service to the table
    rows.extend(
        services
            .iter()
            .filter_map(|(service, layer_name)| build_service_row(service, layer_name)),
    );

    // Add jobs from all_job_info (already filtered for this functional class)
    rows.extend(
        all_job_info
            .iter()
            .map(|(job_name, layer_name)| build_job_row(job_name, layer_name)),
    );

    let total_count = rows.len();

    // If no services or jobs, show a message
    if total_count == 0 {
        rows.push(DetailRow::normal(
            vec![
                DetailCell::text("No services or jobs in this functional class"),
                DetailCell::text("-"),
                DetailCell::text("-"),
                DetailCell::text("-"),
                DetailCell::text("-"),
                DetailCell::text("-"),
            ],
            0,
        ));
    }

    sections.push(
        DetailSectionData::new(
            format!("Services and Jobs ({total_count})"),
            DetailContent::Table {
                header,
                rows,
                constraints: vec![
                    ColumnConstraint::Percentage(25), // ShortName
                    ColumnConstraint::Percentage(10), // Type
                    ColumnConstraint::Percentage(15), // SID_RQ
                    ColumnConstraint::Percentage(20), // Semantic
                    ColumnConstraint::Percentage(15), // Addressing
                    ColumnConstraint::Percentage(15), // Layer
                ],
                use_row_selection: false,
            },
            false,
        )
        .with_type(DetailSectionType::Services),
    );

    sections
}
