/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::{DiagLayer, DiagService, ParamType, Parameter, ParentRef};

// Import rendering functions from specialized modules
use super::{
    requests::build_request_section,
    responses::{build_neg_responses_sections, build_pos_responses_sections},
};
use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ColumnConstraint, DetailContent, DetailRow, DetailRowType, DetailSectionData,
        DetailSectionType, NodeType,
    },
};

// Helper functions to extract parameter data
pub fn extract_coded_value(param: &Parameter<'_>) -> String {
    let Ok(pt) = param.param_type() else {
        return String::new();
    };

    if !matches!(pt, ParamType::CodedConst) {
        return String::new();
    }

    param
        .specific_data_as_coded_const()
        .and_then(|cc| cc.coded_value())
        .map(|v| {
            if let Ok(num) = v.parse::<u64>() {
                // Format with minimum 2 hex digits (0x01, 0x10, 0x100, etc.)
                if num <= 0xFF {
                    format!("0x{num:02X}")
                } else if num <= 0xFFFF {
                    format!("0x{num:04X}")
                } else if num <= 0xFFFFFF {
                    format!("0x{num:06X}")
                } else if num <= 0xFFFFFFFF {
                    format!("0x{num:08X}")
                } else {
                    format!("0x{num:016X}")
                }
            } else {
                v.to_owned()
            }
        })
        .unwrap_or_default()
}

pub fn extract_dop_name(param: &Parameter<'_>) -> String {
    let Ok(pt) = param.param_type() else {
        return String::new();
    };

    if !matches!(pt, ParamType::Value) {
        return String::new();
    }

    param
        .specific_data_as_value()
        .and_then(|vd| vd.dop())
        .and_then(|dop| dop.short_name())
        .map(|s| s.to_owned())
        .unwrap_or_default()
}

/// Add Diag-Comms section (services and jobs) to the tree
pub fn add_diag_comms<'a>(
    b: &mut TreeBuilder,
    layer: &DiagLayer<'a>,
    depth: usize,
    _layer_name: &str,
    variant_parent_refs: Option<impl Iterator<Item = ParentRef<'a>> + 'a>,
) {
    // Collect own services
    let mut own_services: Vec<DiagService<'_>> = layer
        .diag_services()
        .map(|services| services.iter().map(DiagService).collect())
        .unwrap_or_default();

    // Sort own services alphabetically by name
    own_services.sort_by_cached_key(|ds| {
        ds.diag_comm()
            .and_then(|dc| dc.short_name())
            .unwrap_or("")
            .to_lowercase()
    });

    // Collect services from parent refs with source layer names
    let mut parent_services: Vec<(DiagService<'_>, String)> =
        if let Some(parent_refs) = variant_parent_refs {
            get_parent_ref_services_recursive(parent_refs)
        } else {
            Vec::new()
        };

    // Sort parent services alphabetically by name
    parent_services.sort_by_cached_key(|(ds, _)| {
        ds.diag_comm()
            .and_then(|dc| dc.short_name())
            .unwrap_or("")
            .to_lowercase()
    });

    // Count single ECU jobs
    let job_count = layer.single_ecu_jobs().map(|jobs| jobs.len()).unwrap_or(0);

    let service_count = own_services.len() + parent_services.len();
    let total_count = service_count + job_count;

    if total_count > 0 {
        // Build detail section for Diag-Comms header showing all services and jobs
        // Collect job names from diag_com
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

        // Sort job names alphabetically
        job_names.sort_by_key(|name| name.to_lowercase());
        let detail_section =
            build_diag_comms_table_section(&own_services, &parent_services, &job_names);

        b.push_service_list_header(
            depth,
            format!(
                "Diag-Comms ({} services, {} jobs)",
                service_count, job_count
            ),
            false,
            true,
            vec![detail_section],
            crate::tree::ServiceListType::DiagComms,
        );

        // Add own services first
        for ds in own_services.iter() {
            let Some(display_name) = super::format_service_display_name(ds) else {
                continue;
            };
            let sections = build_diag_comm_details_with_parent(ds, None);

            b.push_details_structured(
                depth + 1,
                format!("[Service] {}", display_name),
                false,
                false,
                sections,
                NodeType::Service,
            );
        }

        // Add parent ref services with different node type
        for (ds, source_layer_name) in parent_services.iter() {
            let Some(display_name) = super::format_service_display_name(ds) else {
                continue;
            };
            let sections = build_diag_comm_details_with_parent(ds, Some(source_layer_name.clone()));

            b.push_details_structured(
                depth + 1,
                format!("[Service] {}", display_name),
                false,
                false,
                sections,
                NodeType::ParentRefService, // Mark as inherited
            );
        }

        // Add single ECU jobs after services
        if let Some(jobs) = layer.single_ecu_jobs() {
            // Sort jobs alphabetically by name
            let mut sorted_jobs: Vec<_> = jobs.iter().collect();
            sorted_jobs.sort_by_cached_key(|job| {
                job.diag_comm()
                    .and_then(|dc| dc.short_name())
                    .unwrap_or("")
                    .to_lowercase()
            });

            for job in sorted_jobs.into_iter() {
                let job_name = job
                    .diag_comm()
                    .and_then(|dc| dc.short_name())
                    .unwrap_or("Unnamed");

                // Build job details inline
                let mut sections = Vec::new();

                // Add header section with job name
                sections.push(DetailSectionData {
                    title: format!("Job - {}", job_name),
                    render_as_header: true,
                    content: DetailContent::PlainText(vec![]),
                    section_type: DetailSectionType::Header,
                });

                // Overview section
                let header = DetailRow {
                    row_type: DetailRowType::Normal,
                    metadata: None,
                    cells: vec!["Property".to_owned(), "Value".to_owned()],
                    cell_types: vec![CellType::Text, CellType::Text],
                    indent: 0,
                };

                let rows = vec![
                    DetailRow {
                        row_type: DetailRowType::Normal,
                        metadata: None,
                        cells: vec!["Job Name".to_owned(), job_name.to_owned()],
                        cell_types: vec![CellType::Text, CellType::Text],
                        indent: 0,
                    },
                    DetailRow {
                        row_type: DetailRowType::Normal,
                        metadata: None,
                        cells: vec!["Status".to_owned(), "Available in database".to_owned()],
                        cell_types: vec![CellType::Text, CellType::Text],
                        indent: 0,
                    },
                ];

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

                // Precondition State Refs
                if let Some(dc) = job.diag_comm() {
                    let header = DetailRow {
                        cells: vec![
                            "State".to_owned(),
                            "Value".to_owned(),
                            "Input Param".to_owned(),
                        ],
                        cell_types: vec![CellType::Text, CellType::Text, CellType::Text],
                        indent: 0,
                        ..Default::default()
                    };

                    let mut rows: Vec<DetailRow> = dc
                        .pre_condition_state_refs()
                        .into_iter()
                        .flat_map(|refs| refs.iter())
                        .map(|pc| {
                            let state_name = pc
                                .state()
                                .and_then(|s| s.short_name())
                                .unwrap_or("-")
                                .to_owned();
                            let value = pc.value().unwrap_or("-").to_owned();
                            let input_param = pc
                                .in_param_if_short_name()
                                .or_else(|| pc.in_param_path_short_name())
                                .unwrap_or("-")
                                .to_owned();

                            DetailRow {
                                cells: vec![state_name, value, input_param],
                                cell_types: vec![CellType::Text, CellType::Text, CellType::Text],
                                indent: 0,
                                ..Default::default()
                            }
                        })
                        .collect();

                    if rows.is_empty() {
                        rows.push(DetailRow {
                            cells: vec![
                                "(No precondition state refs)".to_owned(),
                                "-".to_owned(),
                                "-".to_owned(),
                            ],
                            cell_types: vec![CellType::Text, CellType::Text, CellType::Text],
                            indent: 0,
                            ..Default::default()
                        });
                    }

                    sections.push(DetailSectionData {
                        title: "Precondition-State-Refs".to_owned(),
                        render_as_header: false,
                        section_type: DetailSectionType::Custom,
                        content: DetailContent::Table {
                            header,
                            rows,
                            constraints: vec![
                                ColumnConstraint::Percentage(40),
                                ColumnConstraint::Percentage(30),
                                ColumnConstraint::Percentage(30),
                            ],
                            use_row_selection: true,
                        },
                    });
                }

                b.push_details_structured(
                    depth + 1,
                    format!("[Job] {}", job_name),
                    false,
                    false,
                    sections,
                    NodeType::Job,
                );
            }
        }
    }
}

/// Get services from parent references recursively with proper filtering
/// Returns a tuple of (DiagService, source_layer_name)
pub fn get_parent_ref_services_recursive<'a>(
    parent_refs: impl Iterator<Item = ParentRef<'a>>,
) -> Vec<(DiagService<'a>, String)> {
    fn filter_not_inherited_services<'a>(
        diag_services: impl Iterator<Item = impl Into<DiagService<'a>>>,
        not_inherited_names: &[&str],
        source_layer_name: String,
    ) -> Vec<(DiagService<'a>, String)> {
        diag_services
            .into_iter()
            .map(|s| s.into())
            .filter(|service| {
                service
                    .diag_comm()
                    .and_then(|dc| dc.short_name())
                    .is_none_or(|name| !not_inherited_names.contains(&name))
            })
            .map(|service| (service, source_layer_name.clone()))
            .collect()
    }

    fn find_services_recursive<'a>(
        parent_refs: impl Iterator<Item = ParentRef<'a>>,
    ) -> Vec<(DiagService<'a>, String)> {
        parent_refs
            .into_iter()
            .filter_map(|parent_ref| {
                // Get the list of short names that should not be inherited
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
                            layer_name,
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
                            layer_name,
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
                            layer_name,
                        ))
                    }
                    _ => {
                        // Unsupported parent ref type
                        None
                    }
                }
            })
            .flatten()
            .collect()
    }

    find_services_recursive(parent_refs)
}

/// Build detailed sections for a diagnostic service with optional parent layer info
pub fn build_diag_comm_details_with_parent(
    ds: &DiagService<'_>,
    parent_layer_name: Option<String>,
) -> Vec<DetailSectionData> {
    let mut sections: Vec<DetailSectionData> = Vec::new();

    // Add header section with service ID and name (matching tree display format)
    let service_name = ds.diag_comm().and_then(|dc| dc.short_name()).unwrap_or("?");
    let id_str = super::format_service_id(ds);
    let header_title = if id_str.is_empty() {
        format!("Service - {}", service_name)
    } else {
        format!("Service - {} - {}", id_str, service_name)
    };

    sections.push(DetailSectionData {
        title: header_title,
        render_as_header: true,
        content: DetailContent::PlainText(vec![]),
        section_type: DetailSectionType::Header,
    });

    // Overview - table with key-value pairs
    let header = DetailRow::header(
        vec!["Property".to_owned(), "Value".to_owned()],
        vec![CellType::Text, CellType::Text],
    );

    let mut rows = Vec::new();

    if let Some(dc) = ds.diag_comm() {
        rows.extend(dc.short_name().map(|sn| {
            DetailRow::normal(
                vec!["Service".to_owned(), sn.to_owned()],
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
    }
    if let Some(sid) = ds.request_id() {
        rows.push(DetailRow::normal(
            vec!["SID".to_owned(), format!("0x{sid:02X}")],
            vec![CellType::Text, CellType::Text],
            0,
        ));
    }
    if let Some((sub_fn, bit_len)) = ds.request_sub_function_id() {
        rows.push(DetailRow::normal(
            vec![
                "Sub-Function".to_owned(),
                format!("0x{sub_fn:04X} ({bit_len} bits)"),
            ],
            vec![CellType::Text, CellType::Text],
            0,
        ));
    }
    rows.push(DetailRow::normal(
        vec!["Addressing".to_owned(), format!("{:?}", ds.addressing())],
        vec![CellType::Text, CellType::Text],
        0,
    ));
    rows.push(DetailRow::normal(
        vec![
            "Transmission".to_owned(),
            format!("{:?}", ds.transmission_mode()),
        ],
        vec![CellType::Text, CellType::Text],
        0,
    ));

    // Add inheritance information only if inherited
    if let Some(parent_name) = parent_layer_name {
        rows.push(DetailRow::inherited_from(parent_name));
    }

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
                use_row_selection: true, // Use row selection for Overview
            },
            false,
        )
        .with_type(DetailSectionType::Overview),
    );

    // Request params - use rendering logic from requests module
    if let Some(request_section) = build_request_section(ds) {
        sections.push(request_section);
    }

    // Pos responses - use rendering logic from responses module
    sections.extend(build_pos_responses_sections(ds));

    // Neg responses - use rendering logic from responses module
    sections.extend(build_neg_responses_sections(ds));

    // ComParam refs
    let comparam_header = DetailRow {
        row_type: DetailRowType::Normal,
        metadata: None,
        cells: vec![
            "ComParam".to_owned(),
            "Value".to_owned(),
            "Complex Value".to_owned(),
            "Protocol".to_owned(),
            "Prot-Stack".to_owned(),
        ],
        cell_types: vec![
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
        ],
        indent: 0,
    };
    sections.push(DetailSectionData {
        title: "ComParam-Refs".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::ComParams,
        content: DetailContent::Table {
            header: comparam_header,
            rows: vec![DetailRow {
                row_type: DetailRowType::Normal,
                metadata: None,
                cells: vec!["(No ComParam refs at comm level)".to_owned()],
                cell_types: vec![CellType::Text],
                indent: 0,
            }],
            constraints: vec![
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(20),
            ],
            use_row_selection: false,
        },
    });

    // Audience section - Combined into single tab with composite content
    let audience_section = ds
        .diag_comm()
        .and_then(|dc| dc.audience())
        .map(|audience| {
            let flag_lines = vec![
                format!("IS_MANUFACTURER: {}", audience.is_manufacturing()),
                format!("IS_DEVELOPMENT: {}", audience.is_development()),
                format!("IS_AFTERSALES: {}", audience.is_after_sales()),
                format!("IS_AFTERMARKET: {}", audience.is_after_market()),
            ];

            let mut subsections = vec![DetailSectionData {
                title: "Audience Flags".to_owned(),
                render_as_header: false,
                section_type: DetailSectionType::Custom,
                content: DetailContent::PlainText(flag_lines),
            }];

            // Additional audiences subsection
            let audiences_list: Vec<_> = audience
                .enabled_audiences()
                .into_iter()
                .flat_map(|a| a.iter())
                .filter_map(|aa| aa.short_name().map(|s| s.to_owned()))
                .collect();

            if !audiences_list.is_empty() {
                subsections.push(DetailSectionData {
                    title: "Additional Audiences".to_owned(),
                    render_as_header: false,
                    section_type: DetailSectionType::Custom,
                    content: DetailContent::PlainText(audiences_list),
                });
            }

            DetailSectionData {
                title: "Audience".to_owned(),
                render_as_header: false,
                section_type: DetailSectionType::Custom,
                content: DetailContent::Composite(subsections),
            }
        })
        .unwrap_or_else(|| DetailSectionData {
            title: "Audience".to_owned(),
            render_as_header: false,
            section_type: DetailSectionType::Custom,
            content: DetailContent::PlainText(vec!["(No audience info)".to_owned()]),
        });
    sections.push(audience_section);

    // SDGs - Get from DiagComm
    {
        let mut sdg_rows: Vec<DetailRow> = ds
            .diag_comm()
            .and_then(|dc| dc.sdgs())
            .and_then(|sdgs| sdgs.sdgs())
            .into_iter()
            .flat_map(|sdg_list| sdg_list.iter())
            .filter_map(|sdg| {
                let caption = sdg.caption_sn()?;
                let si = sdg.si().unwrap_or("-");

                let mut rows = vec![DetailRow::normal(
                    vec![
                        format!("SDG: {}", caption),
                        si.to_owned(),
                        String::new(),
                        String::new(),
                    ],
                    vec![
                        CellType::Text,
                        CellType::Text,
                        CellType::Text,
                        CellType::Text,
                    ],
                    0,
                )];

                // Add nested SD items
                rows.extend(
                    sdg.sds()
                        .into_iter()
                        .flat_map(|sds| sds.iter())
                        .filter_map(|entry| entry.sd_or_sdg_as_sd())
                        .map(|sd| {
                            DetailRow::normal(
                                vec![
                                    format!("  SD: {}", sd.value().unwrap_or("-")),
                                    sd.si().unwrap_or("-").to_owned(),
                                    sd.ti().unwrap_or("-").to_owned(),
                                    String::new(),
                                ],
                                vec![
                                    CellType::Text,
                                    CellType::Text,
                                    CellType::Text,
                                    CellType::Text,
                                ],
                                1,
                            )
                        }),
                );

                Some(rows)
            })
            .flatten()
            .collect();

        if sdg_rows.is_empty() {
            sdg_rows.push(DetailRow::normal(
                vec!["(No SDGs available)".to_owned()],
                vec![CellType::Text],
                0,
            ));
        }

        let sdg_header = DetailRow::header(
            vec![
                "Caption/Value".to_owned(),
                "SI".to_owned(),
                "TI".to_owned(),
                "".to_owned(),
            ],
            vec![
                CellType::Text,
                CellType::Text,
                CellType::Text,
                CellType::Text,
            ],
        );
        sections.push(DetailSectionData {
            title: "SDGs".to_owned(),
            render_as_header: false,
            section_type: DetailSectionType::Custom,
            content: DetailContent::Table {
                header: sdg_header,
                rows: sdg_rows,
                constraints: vec![
                    ColumnConstraint::Percentage(50),
                    ColumnConstraint::Percentage(20),
                    ColumnConstraint::Percentage(20),
                    ColumnConstraint::Percentage(10),
                ],
                use_row_selection: true,
            },
        });
    }

    // Precondition State Refs
    sections.push(build_precondition_state_refs_section(ds));

    // State Transition Refs
    sections.push(build_state_transition_refs_section(ds));

    // Related diag comm refs
    let related_header = DetailRow {
        row_type: DetailRowType::Normal,
        metadata: None,
        cells: vec!["Short Name".to_owned()],
        cell_types: vec![CellType::Text],
        indent: 0,
    };
    sections.push(DetailSectionData {
        title: "Related-Diag-Comm-Refs".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::RelatedRefs,
        content: DetailContent::Table {
            header: related_header,
            rows: vec![DetailRow {
                row_type: DetailRowType::Normal,
                metadata: None,
                cells: vec!["(Related comms not available)".to_owned()],
                cell_types: vec![CellType::Text],
                indent: 0,
            }],
            constraints: vec![ColumnConstraint::Percentage(100)],
            use_row_selection: false,
        },
    });

    sections
}
/// Build a table section for the Diag-Comms header showing all services and jobs
fn build_diag_comms_table_section(
    own_services: &[DiagService<'_>],
    parent_services: &[(DiagService<'_>, String)],
    job_names: &[String],
) -> DetailSectionData {
    let header = DetailRow {
        row_type: DetailRowType::Normal,
        metadata: None,
        cells: vec![
            "Short Name".to_owned(),
            "ID".to_owned(),
            "Funct Class".to_owned(),
            "Type".to_owned(),
            "Inherited".to_owned(),
        ],
        cell_types: vec![
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
        ],
        indent: 0,
    };

    let mut rows = Vec::new();

    // Helper: build a row for a service with given inherited flag
    let build_service_row = |ds: &DiagService<'_>, inherited: &str| -> Option<DetailRow> {
        let dc = ds.diag_comm()?;
        let name = dc.short_name().unwrap_or("?").to_owned();
        let id_str = super::format_service_id(ds);
        let id = if id_str.is_empty() {
            "-".to_owned()
        } else {
            id_str
        };

        let funct_class = dc
            .funct_class()
            .map(|fc_list| fc_list.get(0))
            .and_then(|fc| fc.short_name())
            .unwrap_or("-")
            .to_owned();

        Some(DetailRow {
            row_type: DetailRowType::Normal,
            metadata: None,
            cells: vec![
                name,
                id,
                funct_class,
                "Service".to_owned(),
                inherited.to_owned(),
            ],
            cell_types: vec![
                CellType::Text,
                CellType::Text,
                CellType::Text,
                CellType::Text,
                CellType::Text,
            ],
            indent: 0,
        })
    };

    // Add own services first (inherited = false)
    rows.extend(
        own_services
            .iter()
            .filter_map(|ds| build_service_row(ds, "false")),
    );

    // Add parent services (inherited = true)
    rows.extend(
        parent_services
            .iter()
            .filter_map(|(ds, _)| build_service_row(ds, "true")),
    );

    // Add job entries to the table
    for job_name in job_names.iter() {
        rows.push(DetailRow {
            row_type: DetailRowType::Normal,
            metadata: None,
            cells: vec![
                job_name.clone(),
                "-".to_owned(),
                "-".to_owned(),
                "Job".to_owned(),
                "false".to_owned(),
            ],
            cell_types: vec![
                CellType::Text,
                CellType::Text,
                CellType::Text,
                CellType::Text,
                CellType::Text,
            ],
            indent: 0,
        });
    }

    DetailSectionData {
        title: format!(
            "Diag-Comms ({} services, {} jobs)",
            own_services.len() + parent_services.len(),
            job_names.len()
        ),
        render_as_header: false,
        section_type: DetailSectionType::Services,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(35),
                ColumnConstraint::Percentage(12),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(13),
                ColumnConstraint::Percentage(20),
            ],
            use_row_selection: true,
        },
    }
}

/// Build precondition state refs section from a DiagService.
/// Extracts data from the embedded State object rather than just the value string.
fn build_precondition_state_refs_section(ds: &DiagService<'_>) -> DetailSectionData {
    let header = DetailRow {
        cells: vec![
            "State".to_owned(),
            "Value".to_owned(),
            "Input Param".to_owned(),
        ],
        cell_types: vec![CellType::Text, CellType::Text, CellType::Text],
        indent: 0,
        ..Default::default()
    };

    let rows: Vec<DetailRow> = ds
        .diag_comm()
        .and_then(|dc| dc.pre_condition_state_refs())
        .into_iter()
        .flat_map(|refs| refs.iter())
        .map(|pc| {
            let state_name = pc
                .state()
                .and_then(|s| s.short_name())
                .unwrap_or("-")
                .to_owned();
            let value = pc.value().unwrap_or("-").to_owned();
            let input_param = pc
                .in_param_if_short_name()
                .or_else(|| pc.in_param_path_short_name())
                .unwrap_or("-")
                .to_owned();

            DetailRow {
                cells: vec![state_name, value, input_param],
                cell_types: vec![CellType::Text, CellType::Text, CellType::Text],
                indent: 0,
                ..Default::default()
            }
        })
        .collect();

    let rows = if rows.is_empty() {
        vec![DetailRow {
            cells: vec![
                "(No precondition state refs)".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
            ],
            cell_types: vec![CellType::Text, CellType::Text, CellType::Text],
            indent: 0,
            ..Default::default()
        }]
    } else {
        rows
    };

    DetailSectionData {
        title: "Precondition-State-Refs".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Custom,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(40),
                ColumnConstraint::Percentage(30),
                ColumnConstraint::Percentage(30),
            ],
            use_row_selection: true,
        },
    }
}

/// Build state transition refs section from a DiagService.
/// Extracts data from the embedded StateTransition object.
fn build_state_transition_refs_section(ds: &DiagService<'_>) -> DetailSectionData {
    let header = DetailRow {
        cells: vec![
            "Short Name".to_owned(),
            "Source".to_owned(),
            "Target".to_owned(),
            "Value".to_owned(),
        ],
        cell_types: vec![
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
        ],
        indent: 0,
        ..Default::default()
    };

    let rows: Vec<DetailRow> = ds
        .diag_comm()
        .and_then(|dc| dc.state_transition_refs())
        .into_iter()
        .flat_map(|refs| refs.iter())
        .map(|st| {
            let (short_name, source, target) = st
                .state_transition()
                .map(|t| {
                    (
                        t.short_name().unwrap_or("-").to_owned(),
                        t.source_short_name_ref().unwrap_or("-").to_owned(),
                        t.target_short_name_ref().unwrap_or("-").to_owned(),
                    )
                })
                .unwrap_or_else(|| ("-".to_owned(), "-".to_owned(), "-".to_owned()));
            let value = st.value().unwrap_or("-").to_owned();

            DetailRow {
                cells: vec![short_name, source, target, value],
                cell_types: vec![
                    CellType::Text,
                    CellType::Text,
                    CellType::Text,
                    CellType::Text,
                ],
                indent: 0,
                ..Default::default()
            }
        })
        .collect();

    let rows = if rows.is_empty() {
        vec![DetailRow {
            cells: vec![
                "(No state transition refs)".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
            ],
            cell_types: vec![
                CellType::Text,
                CellType::Text,
                CellType::Text,
                CellType::Text,
            ],
            indent: 0,
            ..Default::default()
        }]
    } else {
        rows
    };

    DetailSectionData {
        title: "State-Transition-Refs".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::States,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(30),
                ColumnConstraint::Percentage(25),
                ColumnConstraint::Percentage(25),
                ColumnConstraint::Percentage(20),
            ],
            use_row_selection: true,
        },
    }
}
