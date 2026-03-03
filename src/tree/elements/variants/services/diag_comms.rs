/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::{DiagComm, DiagLayer, DiagService, ParentRef};

use super::details::{
    build_diag_comm_details_with_parent, build_diag_comms_table_section,
    build_precondition_state_refs_from_diag_comm,
};
use crate::tree::{
    builder::TreeBuilder,
    elements::variants::format_service_display_name,
    types::{
        CellType, ColumnConstraint, DetailContent, DetailRow, DetailSectionData, DetailSectionType,
        NodeTextPrefix, NodeType,
    },
};

pub fn add_diag_comms<'a>(
    b: &mut TreeBuilder,
    layer: &DiagLayer<'a>,
    depth: usize,
    variant_parent_refs: Option<impl Iterator<Item = ParentRef<'a>> + 'a>,
) {
    let mut own_services: Vec<DiagService<'_>> = layer
        .diag_services()
        .map(|services| services.iter().map(DiagService).collect())
        .unwrap_or_default();

    own_services.sort_by_cached_key(|ds| {
        ds.diag_comm()
            .and_then(|dc| dc.short_name())
            .unwrap_or("")
            .to_lowercase()
    });

    let mut parent_services: Vec<(DiagService<'_>, String)> =
        if let Some(parent_refs) = variant_parent_refs {
            get_parent_ref_services_recursive(parent_refs)
        } else {
            Vec::new()
        };

    parent_services.sort_by_cached_key(|(ds, _)| {
        ds.diag_comm()
            .and_then(|dc| dc.short_name())
            .unwrap_or("")
            .to_lowercase()
    });

    let job_count = layer.single_ecu_jobs().map_or(0, |jobs| jobs.len());
    let service_count = own_services.len().saturating_add(parent_services.len());
    let total_count = service_count.saturating_add(job_count);

    if total_count > 0 {
        let mut job_names: Vec<String> = layer
            .single_ecu_jobs()
            .map(|jobs| {
                jobs.iter()
                    .map(|job| {
                        job.diag_comm()
                            .and_then(|dc| dc.short_name())
                            .unwrap_or("Unnamed")
                            .to_owned()
                    })
                    .collect()
            })
            .unwrap_or_default();

        job_names.sort_by_key(|name| name.to_lowercase());
        let detail_section =
            build_diag_comms_table_section(&own_services, &parent_services, &job_names);

        b.push_service_list_header(
            depth,
            format!("Diag-Comms ({service_count} services, {job_count} jobs)"),
            false,
            true,
            vec![detail_section],
            crate::tree::ServiceListType::DiagComms,
        );

        for ds in &own_services {
            let Some(display_name) = format_service_display_name(ds) else {
                continue;
            };
            let sections = build_diag_comm_details_with_parent(ds, None);

            b.push_details_structured(
                depth.saturating_add(1),
                format!("{}{display_name}", NodeTextPrefix::Service.as_str()),
                false,
                false,
                sections,
                NodeType::Service,
            );
        }

        for (ds, source_layer_name) in &parent_services {
            let Some(display_name) = format_service_display_name(ds) else {
                continue;
            };
            let sections = build_diag_comm_details_with_parent(ds, Some(source_layer_name.clone()));

            b.push_details_structured(
                depth.saturating_add(1),
                format!("{}{display_name}", NodeTextPrefix::Service.as_str()),
                false,
                false,
                sections,
                NodeType::ParentRefService,
            );
        }

        add_single_ecu_jobs(b, layer, depth);
    }
}

fn add_single_ecu_jobs(b: &mut TreeBuilder, layer: &DiagLayer<'_>, depth: usize) {
    let Some(jobs) = layer.single_ecu_jobs() else {
        return;
    };

    let mut sorted_jobs: Vec<_> = jobs.iter().collect();
    sorted_jobs.sort_by_cached_key(|job| {
        job.diag_comm()
            .and_then(|dc| dc.short_name())
            .unwrap_or("")
            .to_lowercase()
    });

    for job in sorted_jobs {
        let Some(dc) = job.diag_comm() else {
            continue;
        };

        let Some(short_name) = dc.short_name() else {
            continue;
        };

        let mut sections = Vec::new();

        sections.push(DetailSectionData {
            title: format!("Job - {short_name}"),
            render_as_header: true,
            content: DetailContent::PlainText(vec![]),
            section_type: DetailSectionType::Header,
        });

        let header = DetailRow::header(
            vec!["Property".to_owned(), "Value".to_owned()],
            vec![CellType::Text, CellType::Text],
        );

        let mut rows = Vec::new();

        rows.extend(dc.short_name().map(|sn| {
            DetailRow::normal(
                vec!["Job".to_owned(), sn.to_owned()],
                vec![CellType::Text, CellType::Text],
                0,
            )
        }));
        rows.extend(dc.semantic().map(|semantic| {
            DetailRow::normal(
                vec!["Semantic".to_owned(), semantic.to_owned()],
                vec![CellType::Text, CellType::Text],
                0,
            )
        }));

        sections.push(
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
            .with_type(DetailSectionType::Overview),
        );

        sections.push(build_precondition_state_refs_from_diag_comm(Some(
            DiagComm(dc),
        )));

        b.push_details_structured(
            depth.saturating_add(1),
            format!("{}{short_name}", NodeTextPrefix::Job.as_str()),
            false,
            false,
            sections,
            NodeType::Job,
        );
    }
}

/// Recursively resolve services from parent references, returning each
/// service paired with the name of the layer it was inherited from.
/// Deduplicates services by short name, keeping the first occurrence.
pub fn get_parent_ref_services_recursive<'a>(
    parent_refs: impl Iterator<Item = ParentRef<'a>>,
) -> Vec<(DiagService<'a>, String)> {
    fn filter_not_inherited_services<'a>(
        diag_services: impl Iterator<Item = impl Into<DiagService<'a>>>,
        not_inherited_names: &[&str],
        source_layer_name: &str,
    ) -> Vec<(DiagService<'a>, String)> {
        diag_services
            .into_iter()
            .map(Into::into)
            .filter(|service| {
                service
                    .diag_comm()
                    .and_then(|dc| dc.short_name())
                    .is_none_or(|name| !not_inherited_names.contains(&name))
            })
            .map(|service| (service, source_layer_name.to_owned()))
            .collect()
    }

    fn find_services_recursive<'a>(
        parent_refs: impl Iterator<Item = ParentRef<'a>>,
    ) -> Vec<(DiagService<'a>, String)> {
        parent_refs
            .into_iter()
            .filter_map(|parent_ref| {
                let not_inherited_names: Vec<&str> = parent_ref
                    .not_inherited_diag_comm_short_names()
                    .map(|names| names.iter().collect())
                    .unwrap_or_default();

                match parent_ref.ref_type().try_into() {
                    Ok(cda_database::datatypes::ParentRefType::EcuSharedData) => {
                        let ecu_shared_data = parent_ref.ref__as_ecu_shared_data()?;
                        let layer = ecu_shared_data.diag_layer()?;
                        let layer_name = layer.short_name().unwrap_or("EcuSharedData").to_owned();
                        let services = layer.diag_services()?.iter().map(DiagService);
                        Some(filter_not_inherited_services(
                            services,
                            &not_inherited_names,
                            &layer_name,
                        ))
                    }
                    Ok(cda_database::datatypes::ParentRefType::FunctionalGroup) => parent_ref
                        .ref__as_functional_group()
                        .and_then(|fg| fg.parent_refs())
                        .map(|nested_refs| {
                            find_services_recursive(nested_refs.iter().map(ParentRef))
                        }),
                    Ok(cda_database::datatypes::ParentRefType::Protocol) => {
                        let protocol = parent_ref.ref__as_protocol()?;
                        let layer = protocol.diag_layer()?;
                        let layer_name = layer.short_name().unwrap_or("Protocol").to_owned();
                        let services = layer.diag_services()?.iter().map(DiagService);
                        Some(filter_not_inherited_services(
                            services,
                            &not_inherited_names,
                            &layer_name,
                        ))
                    }
                    Ok(cda_database::datatypes::ParentRefType::Variant) => {
                        let variant = parent_ref.ref__as_variant()?;
                        let layer = variant.diag_layer()?;
                        let layer_name = layer.short_name().unwrap_or("Variant").to_owned();
                        let services = layer.diag_services()?.iter().map(DiagService);
                        Some(filter_not_inherited_services(
                            services,
                            &not_inherited_names,
                            &layer_name,
                        ))
                    }
                    Ok(
                        cda_database::datatypes::ParentRefType::TableDop
                        | cda_database::datatypes::ParentRefType::NONE,
                    )
                    | Err(_) => None,
                }
            })
            .flatten()
            .collect()
    }

    let all_services = find_services_recursive(parent_refs);

    // Deduplicate by service short name, keeping the first occurrence
    let mut seen = std::collections::HashSet::new();
    all_services
        .into_iter()
        .filter(|(service, _)| {
            let short_name = service
                .diag_comm()
                .and_then(|dc| dc.short_name())
                .unwrap_or("");
            seen.insert(short_name.to_owned())
        })
        .collect()
}
