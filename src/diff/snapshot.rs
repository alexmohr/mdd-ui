// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

//! Owned snapshot types that mirror the MDD `FlatBuffers` hierarchy.
//!
//! The `FlatBuffers`-generated types are borrowed references with no `PartialEq`.
//! These owned snapshots enable structural comparison for diff operations.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use cda_database::datatypes::{
    DiagComm, DiagLayer, DiagService, DiagnosticDatabase, ParamType, Parameter, StateChart, Variant,
};

/// Shorthand: convert `Option<&str>` to an owned `String`, defaulting to empty.
fn s(opt: Option<&str>) -> String {
    opt.unwrap_or_default().to_owned()
}

/// Extract `short_name` values from an optional `FlatBuffers` vector whose items
/// expose a `short_name() -> Option<&str>` accessor.
///
/// This is a macro because the raw `JobParam` type is not publicly nameable
/// (it lives in a `pub(crate)` module of `cda_database`). A macro lets us
/// iterate without spelling out the concrete `FlatBuffers` type.
macro_rules! collect_short_names {
    ($opt_vec:expr) => {
        $opt_vec
            .into_iter()
            .flatten()
            .filter_map(|item| item.short_name().map(str::to_owned))
            .collect::<Vec<String>>()
    };
}

/// Format a `ParamType` as a human-readable string.
fn param_type_name(pt: &ParamType) -> &'static str {
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

// ---------------------------------------------------------------------------
// Snapshot types
// ---------------------------------------------------------------------------

/// Top-level ECU snapshot extracted from a `DiagnosticDatabase`.
#[derive(Clone, Debug, PartialEq)]
pub struct EcuSnapshot {
    pub name: String,
    pub version: String,
    pub revision: String,
    pub metadata: Vec<(String, String)>,
    pub variants: BTreeMap<String, VariantSnapshot>,
    pub functional_groups: BTreeMap<String, FunctionalGroupSnapshot>,
    pub dtcs: BTreeMap<String, DtcSnapshot>,
}

/// A single variant (base or derived).
#[derive(Clone, Debug, PartialEq)]
pub struct VariantSnapshot {
    pub is_base_variant: bool,
    pub diag_layer: DiagLayerSnapshot,
}

/// A functional group.
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionalGroupSnapshot {
    pub diag_layer: DiagLayerSnapshot,
}

/// The core diagnostic layer present in variants and functional groups.
#[derive(Clone, Debug, PartialEq)]
pub struct DiagLayerSnapshot {
    pub short_name: String,
    pub long_name: String,
    pub services: BTreeMap<String, DiagServiceSnapshot>,
    pub single_ecu_jobs: BTreeMap<String, SingleEcuJobSnapshot>,
    pub state_charts: BTreeMap<String, StateChartSnapshot>,
    pub funct_classes: Vec<String>,
}

/// A diagnostic service with request/response data.
#[derive(Clone, Debug, PartialEq)]
pub struct DiagServiceSnapshot {
    pub diag_comm: DiagCommSnapshot,
    pub addressing: String,
    pub transmission_mode: String,
    pub is_cyclic: bool,
    pub is_multiple: bool,
    pub request: Option<RequestSnapshot>,
    pub pos_responses: Vec<ResponseSnapshot>,
    pub neg_responses: Vec<ResponseSnapshot>,
}

/// The `DiagComm` portion shared by services and ECU jobs.
#[derive(Clone, Debug, PartialEq)]
pub struct DiagCommSnapshot {
    pub short_name: String,
    pub long_name: String,
    pub semantic: String,
    pub diag_class_type: String,
    pub funct_classes: Vec<String>,
    pub is_mandatory: bool,
    pub is_executable: bool,
    pub is_final: bool,
    pub audience: Option<AudienceSnapshot>,
}

/// A request message (parameter list).
#[derive(Clone, Debug, PartialEq)]
pub struct RequestSnapshot {
    pub params: Vec<ParamSnapshot>,
}

/// A response message (positive or negative).
#[derive(Clone, Debug, PartialEq)]
pub struct ResponseSnapshot {
    pub response_type: String,
    pub params: Vec<ParamSnapshot>,
}

/// A single parameter within a request or response.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamSnapshot {
    pub short_name: String,
    pub semantic: String,
    pub physical_default_value: String,
    pub byte_position: u32,
    pub bit_position: u32,
    pub param_type: String,
    pub specific_data_summary: String,
}

/// A Diagnostic Trouble Code.
#[derive(Clone, Debug, PartialEq)]
pub struct DtcSnapshot {
    pub short_name: String,
    pub trouble_code: u32,
    pub display_trouble_code: String,
    pub text: String,
    pub level: Option<u32>,
    pub is_temporary: bool,
}

/// A state chart with states and transitions.
#[derive(Clone, Debug, PartialEq)]
pub struct StateChartSnapshot {
    pub short_name: String,
    pub semantic: String,
    pub start_state: String,
    pub states: Vec<String>,
    pub transitions: Vec<String>,
}

/// A single-ECU job entry.
#[derive(Clone, Debug, PartialEq)]
pub struct SingleEcuJobSnapshot {
    pub diag_comm: DiagCommSnapshot,
    pub input_params: Vec<String>,
    pub output_params: Vec<String>,
    pub neg_output_params: Vec<String>,
}

/// Audience flags on a `DiagComm`.
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct AudienceSnapshot {
    pub is_supplier: bool,
    pub is_development: bool,
    pub is_manufacturing: bool,
    pub is_after_sales: bool,
    pub is_after_market: bool,
}

// ---------------------------------------------------------------------------
// Extraction: EcuSnapshot (entry point)
// ---------------------------------------------------------------------------

impl EcuSnapshot {
    /// Build an owned snapshot from a loaded `DiagnosticDatabase`.
    pub fn from_database(db: &DiagnosticDatabase) -> Result<Self> {
        let ecu = db.ecu_data().context("Failed to read ECU data")?;
        let name = db.ecu_name().unwrap_or_else(|_| "Unknown ECU".to_owned());

        let version = s(ecu.version());
        let revision = s(ecu.revision());

        let metadata: Vec<(String, String)> = ecu
            .metadata()
            .into_iter()
            .flatten()
            .filter_map(|kv| Some((kv.key()?.to_owned(), kv.value()?.to_owned())))
            .collect();

        let variants = ecu
            .variants()
            .into_iter()
            .flatten()
            .filter_map(|v| {
                let vw = Variant(v);
                let snap = VariantSnapshot::from_variant(&vw);
                let key = snap.diag_layer.short_name.clone();
                (!key.is_empty()).then_some((key, snap))
            })
            .collect();

        let functional_groups = ecu
            .functional_groups()
            .into_iter()
            .flatten()
            .filter_map(|fg| {
                let dl = fg.diag_layer()?;
                let layer = DiagLayer(dl);
                let snap = FunctionalGroupSnapshot::from_functional_group(&layer);
                let key = snap.diag_layer.short_name.clone();
                (!key.is_empty()).then_some((key, snap))
            })
            .collect();

        // DTCs are raw FlatBuffers types without a public wrapper, so we
        // extract them inline where the compiler can infer the type.
        let dtcs = ecu
            .dtcs()
            .into_iter()
            .flatten()
            .filter_map(|dtc| {
                let short_name = s(dtc.short_name());
                let trouble_code = dtc.trouble_code();
                let display_trouble_code = s(dtc.display_trouble_code());
                let text = dtc
                    .text()
                    .and_then(|t| t.value())
                    .unwrap_or_default()
                    .to_owned();
                let level = dtc.level();
                let is_temporary = dtc.is_temporary();

                let snap = DtcSnapshot {
                    short_name: short_name.clone(),
                    trouble_code,
                    display_trouble_code,
                    text,
                    level,
                    is_temporary,
                };
                (!short_name.is_empty()).then_some((short_name, snap))
            })
            .collect();

        Ok(Self {
            name,
            version,
            revision,
            metadata,
            variants,
            functional_groups,
            dtcs,
        })
    }
}

// ---------------------------------------------------------------------------
// Extraction: Variant / FunctionalGroup
// ---------------------------------------------------------------------------

impl VariantSnapshot {
    pub fn from_variant(v: &Variant<'_>) -> Self {
        let is_base_variant = v.is_base_variant();
        let diag_layer = v.diag_layer().map_or_else(DiagLayerSnapshot::empty, |dl| {
            DiagLayerSnapshot::from_layer(&DiagLayer(dl))
        });

        Self {
            is_base_variant,
            diag_layer,
        }
    }
}

impl FunctionalGroupSnapshot {
    pub fn from_functional_group(layer: &DiagLayer<'_>) -> Self {
        Self {
            diag_layer: DiagLayerSnapshot::from_layer(layer),
        }
    }
}

// ---------------------------------------------------------------------------
// Extraction: DiagLayer
// ---------------------------------------------------------------------------

impl DiagLayerSnapshot {
    pub fn from_layer(layer: &DiagLayer<'_>) -> Self {
        let short_name = s(layer.short_name());
        let long_name = layer
            .long_name()
            .and_then(|ln| ln.value())
            .unwrap_or_default()
            .to_owned();

        let services = layer
            .diag_services()
            .into_iter()
            .flatten()
            .filter_map(|ds_raw| {
                let ds = DiagService(ds_raw);
                let snap = DiagServiceSnapshot::from_service(&ds);
                let key = snap.diag_comm.short_name.clone();
                (!key.is_empty()).then_some((key, snap))
            })
            .collect();

        // SingleEcuJob is a raw FlatBuffers type without a public wrapper,
        // so we extract inline where the compiler can infer the type.
        let single_ecu_jobs = layer
            .single_ecu_jobs()
            .into_iter()
            .flatten()
            .filter_map(|job| {
                let diag_comm = job.diag_comm().map_or_else(DiagCommSnapshot::empty, |dc| {
                    DiagCommSnapshot::from_diag_comm(&DiagComm(dc))
                });

                let input_params = collect_short_names!(job.input_params());
                let output_params = collect_short_names!(job.output_params());
                let neg_output_params = collect_short_names!(job.neg_output_params());

                let snap = SingleEcuJobSnapshot {
                    diag_comm,
                    input_params,
                    output_params,
                    neg_output_params,
                };
                let key = snap.diag_comm.short_name.clone();
                (!key.is_empty()).then_some((key, snap))
            })
            .collect();

        let state_charts = layer
            .state_charts()
            .into_iter()
            .flatten()
            .filter_map(|chart| {
                let snap = StateChartSnapshot::from_chart(&StateChart(chart));
                let key = snap.short_name.clone();
                (!key.is_empty()).then_some((key, snap))
            })
            .collect();

        let funct_classes: Vec<String> = layer
            .funct_classes()
            .into_iter()
            .flatten()
            .filter_map(|fc| fc.short_name().map(str::to_owned))
            .collect();

        Self {
            short_name,
            long_name,
            services,
            single_ecu_jobs,
            state_charts,
            funct_classes,
        }
    }

    /// Create an empty placeholder layer.
    fn empty() -> Self {
        Self {
            short_name: String::new(),
            long_name: String::new(),
            services: BTreeMap::new(),
            single_ecu_jobs: BTreeMap::new(),
            state_charts: BTreeMap::new(),
            funct_classes: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Extraction: DiagService
// ---------------------------------------------------------------------------

impl DiagServiceSnapshot {
    pub fn from_service(ds: &DiagService<'_>) -> Self {
        let diag_comm = ds.diag_comm().map_or_else(DiagCommSnapshot::empty, |dc| {
            DiagCommSnapshot::from_diag_comm(&DiagComm(dc))
        });

        let addressing = format!("{:?}", ds.addressing());
        let transmission_mode = format!("{:?}", ds.transmission_mode());
        let is_cyclic = ds.is_cyclic();
        let is_multiple = ds.is_multiple();

        let request = ds.request().map(|req| {
            let params: Vec<ParamSnapshot> = req
                .params()
                .into_iter()
                .flatten()
                .map(|p| ParamSnapshot::from_param(&Parameter(p)))
                .collect();
            RequestSnapshot { params }
        });

        let pos_responses = ds
            .pos_responses()
            .into_iter()
            .flatten()
            .map(|resp| {
                let response_type = format!("{:?}", resp.response_type());
                let params: Vec<ParamSnapshot> = resp
                    .params()
                    .into_iter()
                    .flatten()
                    .map(|p| ParamSnapshot::from_param(&Parameter(p)))
                    .collect();
                ResponseSnapshot {
                    response_type,
                    params,
                }
            })
            .collect();

        let neg_responses = ds
            .neg_responses()
            .into_iter()
            .flatten()
            .map(|resp| {
                let response_type = format!("{:?}", resp.response_type());
                let params: Vec<ParamSnapshot> = resp
                    .params()
                    .into_iter()
                    .flatten()
                    .map(|p| ParamSnapshot::from_param(&Parameter(p)))
                    .collect();
                ResponseSnapshot {
                    response_type,
                    params,
                }
            })
            .collect();

        Self {
            diag_comm,
            addressing,
            transmission_mode,
            is_cyclic,
            is_multiple,
            request,
            pos_responses,
            neg_responses,
        }
    }
}

// ---------------------------------------------------------------------------
// Extraction: DiagComm
// ---------------------------------------------------------------------------

impl DiagCommSnapshot {
    pub fn from_diag_comm(dc: &DiagComm<'_>) -> Self {
        let short_name = s(dc.short_name());
        let long_name = dc
            .long_name()
            .and_then(|ln| ln.value())
            .unwrap_or_default()
            .to_owned();
        let semantic = s(dc.semantic());
        let diag_class_type = format!("{:?}", dc.diag_class_type());

        let funct_classes: Vec<String> = dc
            .funct_class()
            .into_iter()
            .flatten()
            .filter_map(|fc| fc.short_name().map(str::to_owned))
            .collect();

        let is_mandatory = dc.is_mandatory();
        let is_executable = dc.is_executable();
        let is_final = dc.is_final();

        let audience = dc.audience().map(|aud| AudienceSnapshot {
            is_supplier: aud.is_supplier(),
            is_development: aud.is_development(),
            is_manufacturing: aud.is_manufacturing(),
            is_after_sales: aud.is_after_sales(),
            is_after_market: aud.is_after_market(),
        });

        Self {
            short_name,
            long_name,
            semantic,
            diag_class_type,
            funct_classes,
            is_mandatory,
            is_executable,
            is_final,
            audience,
        }
    }

    /// Create an empty placeholder `DiagComm`.
    fn empty() -> Self {
        Self {
            short_name: String::new(),
            long_name: String::new(),
            semantic: String::new(),
            diag_class_type: String::new(),
            funct_classes: Vec::new(),
            is_mandatory: false,
            is_executable: false,
            is_final: false,
            audience: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Extraction: Param
// ---------------------------------------------------------------------------

impl ParamSnapshot {
    pub fn from_param(param: &Parameter<'_>) -> Self {
        let short_name = s(param.short_name());
        let semantic = s(param.semantic());
        let physical_default_value = s(param.physical_default_value());
        let byte_position = param.byte_position();
        let bit_position = param.bit_position();

        let param_type = param.param_type().map_or_else(
            |_| "Unknown".to_owned(),
            |pt| param_type_name(&pt).to_owned(),
        );

        let specific_data_summary = build_specific_data_summary(param);

        Self {
            short_name,
            semantic,
            physical_default_value,
            byte_position,
            bit_position,
            param_type,
            specific_data_summary,
        }
    }
}

/// Build a human-readable summary of the parameter's specific data union.
fn build_specific_data_summary(param: &Parameter<'_>) -> String {
    let Ok(pt) = param.param_type() else {
        return String::new();
    };

    match pt {
        ParamType::CodedConst => {
            let value = param
                .specific_data_as_coded_const()
                .and_then(|cc| cc.coded_value())
                .unwrap_or_default();
            format!("CodedConst({value})")
        }
        ParamType::Value => {
            let dop_name = param
                .specific_data_as_value()
                .and_then(|v| v.dop())
                .and_then(|dop| dop.short_name())
                .unwrap_or_default();
            format!("Value(dop={dop_name})")
        }
        ParamType::PhysConst => {
            let value = param
                .specific_data_as_phys_const()
                .and_then(|pc| pc.phys_constant_value())
                .unwrap_or_default();
            format!("PhysConst({value})")
        }
        ParamType::Reserved => "Reserved".to_owned(),
        ParamType::NrcConst => "NrcConst".to_owned(),
        ParamType::MatchingRequestParam => "MatchingRequestParam".to_owned(),
        ParamType::TableEntry => "TableEntry".to_owned(),
        ParamType::TableKey => "TableKey".to_owned(),
        ParamType::TableStruct => "TableStruct".to_owned(),
        ParamType::System => "System".to_owned(),
        ParamType::Dynamic => "Dynamic".to_owned(),
        ParamType::LengthKey => "LengthKey".to_owned(),
    }
}

// ---------------------------------------------------------------------------
// Extraction: StateChart (using the public wrapper type)
// ---------------------------------------------------------------------------

impl StateChartSnapshot {
    pub fn from_chart(chart: &StateChart<'_>) -> Self {
        let short_name = s(chart.short_name());
        let semantic = s(chart.semantic());
        let start_state = s(chart.start_state_short_name_ref());

        let states: Vec<String> = chart
            .states()
            .into_iter()
            .flatten()
            .filter_map(|st| st.short_name().map(str::to_owned))
            .collect();

        let transitions: Vec<String> = chart
            .state_transitions()
            .into_iter()
            .flatten()
            .map(|tr| {
                let src = tr.source_short_name_ref().unwrap_or("?");
                let tgt = tr.target_short_name_ref().unwrap_or("?");
                format!("{src} -> {tgt}")
            })
            .collect();

        Self {
            short_name,
            semantic,
            start_state,
            states,
            transitions,
        }
    }
}
