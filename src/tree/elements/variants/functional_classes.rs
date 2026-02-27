/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::{DiagLayer, DiagService, Variant};

use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellJumpTarget, CellType, ColumnConstraint, DetailContent, DetailRow, DetailSectionData,
        DetailSectionType, NodeType,
    },
};

/// Collected services (with source layer name) and jobs (name + layer name) for a functional class.
type FcServicesAndJobs<'a> = (Vec<(DiagService<'a>, String)>, Vec<(String, String)>);

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

    // Build table section for the Functional Classes header
    let detail_section = build_functional_classes_table_section(&funct_class_data);

    b.push_service_list_header(
        depth,
        format!("Functional Classes ({count})"),
        false,
        true,
        vec![detail_section],
        crate::tree::ServiceListType::FunctionalClasses,
    );

    // Collect all services and jobs from ALL variants for each functional class
    // We'll do this per functional class below, searching across all variants
    let variants_vec: Vec<Variant<'_>> = all_variants
        .map(std::iter::Iterator::collect)
        .unwrap_or_default();

    // Add each functional class as a child node with its services/jobs from ALL variants
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

        b.push_details_structured(
            depth.saturating_add(1),
            name.clone(),
            false,
            false,
            details,
            NodeType::FunctionalClass,
        );
    }
}

/// Build a table section for the Functional Classes header showing all class definitions
fn build_functional_classes_table_section(items: &[String]) -> DetailSectionData {
    let header = DetailRow::header(vec!["Short Name".to_owned()], vec![CellType::Text]);

    let mut rows = Vec::new();

    // Add each functional class definition to the table
    for name in items {
        rows.push(DetailRow::normal(
            vec![name.clone()],
            vec![CellType::Text],
            0,
        ));
    }

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

/// Collect services and jobs for a specific functional class from a single layer
/// This is used when no variants are provided (e.g., for functional groups or ECU shared data)
fn collect_services_and_jobs_from_layer<'a>(
    fc_name: &str,
    layer: &DiagLayer<'a>,
) -> FcServicesAndJobs<'a> {
    let mut services = Vec::new();
    let mut job_info = Vec::new();

    let layer_name = layer.short_name().unwrap_or("Unknown");

    // Find services in this layer that belong to the functional class
    if let Some(diag_services) = layer.diag_services() {
        for service in diag_services {
            let service_wrap = DiagService(service);
            let Some(dc) = service_wrap.diag_comm() else {
                continue;
            };

            // Check if this service belongs to our functional class
            let belongs_to_fc = dc.funct_class().is_some_and(|funct_classes| {
                funct_classes.iter().any(|fc| {
                    fc.short_name()
                        .is_some_and(|fc_short_name| fc_short_name == fc_name)
                })
            });

            if belongs_to_fc {
                services.push((service_wrap, layer_name.to_string()));
            }
        }
    }

    // Find jobs in this layer that belong to the functional class
    if let Some(ecu_jobs) = layer.single_ecu_jobs() {
        for job in ecu_jobs {
            let Some(job_dc) = job.diag_comm() else {
                continue;
            };

            let Some(short_name) = job_dc.short_name() else {
                continue;
            };

            // Check if this job belongs to our functional class
            let belongs_to_fc = job_dc.funct_class().is_some_and(|funct_classes| {
                funct_classes.iter().any(|fc| {
                    fc.short_name()
                        .is_some_and(|fc_short_name| fc_short_name == fc_name)
                })
            });

            if belongs_to_fc {
                job_info.push((short_name.to_string(), layer_name.to_string()));
            }
        }
    }

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

        // Find services in this variant's layer that belong to the functional class
        if let Some(diag_services) = variant_layer.diag_services() {
            for service in diag_services {
                let service_wrap = DiagService(service);
                let Some(dc) = service_wrap.diag_comm() else {
                    continue;
                };

                let Some(short_name) = dc.short_name() else {
                    continue;
                };

                // Check if this service belongs to our functional class
                let belongs_to_fc = dc.funct_class().is_some_and(|funct_classes| {
                    funct_classes.iter().any(|fc| {
                        fc.short_name()
                            .is_some_and(|fc_short_name| fc_short_name == fc_name)
                    })
                });

                if belongs_to_fc && seen_services.insert(short_name.to_owned()) {
                    services.push((service_wrap, variant_name.to_string()));
                }
            }
        }

        // Find jobs in this variant's layer that belong to the functional class
        if let Some(ecu_jobs) = variant_layer.single_ecu_jobs() {
            for job in ecu_jobs {
                let Some(job_dc) = job.diag_comm() else {
                    continue;
                };

                let Some(short_name) = job_dc.short_name() else {
                    continue;
                };

                // Check if this job belongs to our functional class
                let belongs_to_fc = job_dc.funct_class().is_some_and(|funct_classes| {
                    funct_classes.iter().any(|fc| {
                        fc.short_name()
                            .is_some_and(|fc_short_name| fc_short_name == fc_name)
                    })
                });

                if belongs_to_fc && seen_jobs.insert(short_name.to_owned()) {
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

    let sid_rq = if let Some(sid) = service.request_id() {
        if let Some((sub_fn, bit_len)) = service.request_sub_function_id() {
            let sub_fn_str = if bit_len <= 8 {
                format!("{sub_fn:02X}")
            } else {
                format!("{sub_fn:04X}")
            };
            format!("0x{sid:02X}{sub_fn_str}")
        } else {
            format!("0x{sid:02X}")
        }
    } else {
        "-".to_owned()
    };

    let semantic = dc.semantic().unwrap_or("-").to_owned();
    let addressing = format!("{:?}", service.addressing());

    Some(DetailRow::with_jump_targets(
        vec![
            short_name,
            service_type,
            sid_rq,
            semantic,
            addressing,
            layer_name.to_owned(),
        ],
        vec![
            CellType::ParameterName,
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::ParameterName,
        ],
        vec![
            Some(CellJumpTarget::TreeNodeByName),
            None,
            None,
            None,
            None,
            Some(CellJumpTarget::ContainerByName),
        ],
        0,
    ))
}

fn build_job_row(job_name: &str, layer_name: &str) -> DetailRow {
    DetailRow::with_jump_targets(
        vec![
            job_name.to_owned(),
            "Job".to_owned(),
            "-".to_owned(),
            "-".to_owned(),
            "-".to_owned(),
            layer_name.to_owned(),
        ],
        vec![
            CellType::ParameterName,
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::ParameterName,
        ],
        vec![
            Some(CellJumpTarget::TreeNodeByName),
            None,
            None,
            None,
            None,
            Some(CellJumpTarget::ContainerByName),
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
    });

    // Build services table
    let header = DetailRow::header(
        vec![
            "Short Name".to_owned(),
            "Type".to_owned(),
            "SID_RQ".to_owned(),
            "Semantic".to_owned(),
            "Addressing".to_owned(),
            "Layer".to_owned(),
        ],
        vec![
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
        ],
    );

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
                "No services or jobs in this functional class".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
            ],
            vec![
                CellType::Text,
                CellType::Text,
                CellType::Text,
                CellType::Text,
                CellType::Text,
                CellType::Text,
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
