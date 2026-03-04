// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

//! Structural comparison of two [`EcuSnapshot`]s.
//!
//! The [`compare`] function walks the old and new snapshots in parallel and
//! produces a [`DiffResult`] that describes every addition, removal,
//! modification, and unchanged element at every level of the hierarchy.

use std::collections::BTreeMap;

use crate::{
    diff::snapshot::{
        AudienceSnapshot, DiagCommSnapshot, DiagLayerSnapshot, DiagServiceSnapshot, DtcSnapshot,
        EcuSnapshot, FunctionalGroupSnapshot, ParamSnapshot, ResponseSnapshot,
        SingleEcuJobSnapshot, StateChartSnapshot, VariantSnapshot,
    },
    tree::DiffStatus,
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single property that changed between old and new.
#[derive(Clone, Debug)]
pub struct PropertyDiff {
    pub name: String,
    pub old_value: String,
    pub new_value: String,
}

/// Diff summary counts (top-level elements only).
#[derive(Clone, Debug, Default)]
pub struct DiffSummary {
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
    pub unchanged: usize,
}

/// Diff of a named element at any level of the hierarchy.
#[derive(Clone, Debug)]
pub struct ElementDiff {
    pub name: String,
    pub status: DiffStatus,
    pub property_diffs: Vec<PropertyDiff>,
    pub children: Vec<ElementDiff>,
}

/// Top-level diff result comparing two ECU snapshots.
#[derive(Clone, Debug)]
pub struct DiffResult {
    pub old_name: String,
    pub new_name: String,
    pub ecu_diffs: Vec<PropertyDiff>,
    pub variants: Vec<ElementDiff>,
    pub functional_groups: Vec<ElementDiff>,
    pub dtcs: Vec<ElementDiff>,
    pub summary: DiffSummary,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Compare two [`EcuSnapshot`]s and produce a [`DiffResult`].
pub fn compare(old: &EcuSnapshot, new: &EcuSnapshot) -> DiffResult {
    let ecu_diffs = compare_ecu_properties(old, new);

    let mut summary = DiffSummary::default();

    let variants = compare_maps(&old.variants, &new.variants, compare_variants, &mut summary);
    let functional_groups = compare_maps(
        &old.functional_groups,
        &new.functional_groups,
        compare_functional_groups,
        &mut summary,
    );
    let dtcs = compare_maps(&old.dtcs, &new.dtcs, compare_dtcs, &mut summary);

    DiffResult {
        old_name: old.name.clone(),
        new_name: new.name.clone(),
        ecu_diffs,
        variants,
        functional_groups,
        dtcs,
        summary,
    }
}

// ---------------------------------------------------------------------------
// Generic BTreeMap comparison
// ---------------------------------------------------------------------------

/// Function pointer that compares two values and returns property-level and
/// child-level diffs.
type CompareFn<T> = fn(&T, &T) -> (Vec<PropertyDiff>, Vec<ElementDiff>);

/// Compare two `BTreeMap<String, T>` collections.
///
/// Keys present only in `old` are marked `Removed`, keys only in `new` are
/// marked `Added`, and keys in both are compared with `compare_fn` to decide
/// between `Modified` and `Unchanged`.
///
/// The `summary` counts are updated for **this** level only.
fn compare_maps<T: PartialEq>(
    old: &BTreeMap<String, T>,
    new: &BTreeMap<String, T>,
    compare_fn: CompareFn<T>,
    summary: &mut DiffSummary,
) -> Vec<ElementDiff> {
    let mut results = Vec::new();

    // Removed: keys in old but not in new.
    for key in old.keys() {
        if !new.contains_key(key) {
            summary.removed = summary.removed.saturating_add(1);
            results.push(ElementDiff {
                name: key.clone(),
                status: DiffStatus::Removed,
                property_diffs: Vec::new(),
                children: Vec::new(),
            });
        }
    }

    // Modified or Unchanged: keys in both.
    for (key, old_val) in old {
        if let Some(new_val) = new.get(key) {
            if old_val == new_val {
                summary.unchanged = summary.unchanged.saturating_add(1);
                results.push(ElementDiff {
                    name: key.clone(),
                    status: DiffStatus::Unchanged,
                    property_diffs: Vec::new(),
                    children: Vec::new(),
                });
            } else {
                let (prop_diffs, children) = compare_fn(old_val, new_val);
                summary.modified = summary.modified.saturating_add(1);
                results.push(ElementDiff {
                    name: key.clone(),
                    status: DiffStatus::Modified,
                    property_diffs: prop_diffs,
                    children,
                });
            }
        }
    }

    // Added: keys in new but not in old.
    for key in new.keys() {
        if !old.contains_key(key) {
            summary.added = summary.added.saturating_add(1);
            results.push(ElementDiff {
                name: key.clone(),
                status: DiffStatus::Added,
                property_diffs: Vec::new(),
                children: Vec::new(),
            });
        }
    }

    results
}

// ---------------------------------------------------------------------------
// Property-level diff helpers
// ---------------------------------------------------------------------------

fn diff_prop(name: &str, old: &str, new: &str, diffs: &mut Vec<PropertyDiff>) {
    if old != new {
        diffs.push(PropertyDiff {
            name: name.to_owned(),
            old_value: old.to_owned(),
            new_value: new.to_owned(),
        });
    }
}

fn diff_bool(name: &str, old: bool, new: bool, diffs: &mut Vec<PropertyDiff>) {
    if old != new {
        diffs.push(PropertyDiff {
            name: name.to_owned(),
            old_value: old.to_string(),
            new_value: new.to_string(),
        });
    }
}

fn diff_u32(name: &str, old: u32, new: u32, diffs: &mut Vec<PropertyDiff>) {
    if old != new {
        diffs.push(PropertyDiff {
            name: name.to_owned(),
            old_value: old.to_string(),
            new_value: new.to_string(),
        });
    }
}

fn diff_opt_u32(name: &str, old: Option<u32>, new: Option<u32>, diffs: &mut Vec<PropertyDiff>) {
    if old != new {
        diffs.push(PropertyDiff {
            name: name.to_owned(),
            old_value: old.map_or_else(|| "None".to_owned(), |v| v.to_string()),
            new_value: new.map_or_else(|| "None".to_owned(), |v| v.to_string()),
        });
    }
}

fn diff_string_vec(name: &str, old: &[String], new: &[String], diffs: &mut Vec<PropertyDiff>) {
    if old != new {
        diffs.push(PropertyDiff {
            name: name.to_owned(),
            old_value: old.join(", "),
            new_value: new.join(", "),
        });
    }
}

// ---------------------------------------------------------------------------
// ECU top-level properties
// ---------------------------------------------------------------------------

fn compare_ecu_properties(old: &EcuSnapshot, new: &EcuSnapshot) -> Vec<PropertyDiff> {
    let mut diffs = Vec::new();
    diff_prop("name", &old.name, &new.name, &mut diffs);
    diff_prop("version", &old.version, &new.version, &mut diffs);
    diff_prop("revision", &old.revision, &new.revision, &mut diffs);

    if old.metadata != new.metadata {
        let format_metadata = |m: &[(String, String)]| -> String {
            m.iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; ")
        };
        diffs.push(PropertyDiff {
            name: "metadata".to_owned(),
            old_value: format_metadata(&old.metadata),
            new_value: format_metadata(&new.metadata),
        });
    }

    diffs
}

// ---------------------------------------------------------------------------
// Variant comparison
// ---------------------------------------------------------------------------

fn compare_variants(
    old: &VariantSnapshot,
    new: &VariantSnapshot,
) -> (Vec<PropertyDiff>, Vec<ElementDiff>) {
    let mut diffs = Vec::new();
    diff_bool(
        "is_base_variant",
        old.is_base_variant,
        new.is_base_variant,
        &mut diffs,
    );

    let (layer_props, layer_children) = compare_diag_layers(&old.diag_layer, &new.diag_layer);
    diffs.extend(layer_props);

    (diffs, layer_children)
}

// ---------------------------------------------------------------------------
// Functional group comparison
// ---------------------------------------------------------------------------

fn compare_functional_groups(
    old: &FunctionalGroupSnapshot,
    new: &FunctionalGroupSnapshot,
) -> (Vec<PropertyDiff>, Vec<ElementDiff>) {
    compare_diag_layers(&old.diag_layer, &new.diag_layer)
}

// ---------------------------------------------------------------------------
// DTC comparison
// ---------------------------------------------------------------------------

fn compare_dtcs(old: &DtcSnapshot, new: &DtcSnapshot) -> (Vec<PropertyDiff>, Vec<ElementDiff>) {
    let mut diffs = Vec::new();
    diff_prop("short_name", &old.short_name, &new.short_name, &mut diffs);
    diff_u32(
        "trouble_code",
        old.trouble_code,
        new.trouble_code,
        &mut diffs,
    );
    diff_prop(
        "display_trouble_code",
        &old.display_trouble_code,
        &new.display_trouble_code,
        &mut diffs,
    );
    diff_prop("text", &old.text, &new.text, &mut diffs);
    diff_opt_u32("level", old.level, new.level, &mut diffs);
    diff_bool(
        "is_temporary",
        old.is_temporary,
        new.is_temporary,
        &mut diffs,
    );

    (diffs, Vec::new())
}

// ---------------------------------------------------------------------------
// DiagLayer comparison
// ---------------------------------------------------------------------------

fn compare_diag_layers(
    old: &DiagLayerSnapshot,
    new: &DiagLayerSnapshot,
) -> (Vec<PropertyDiff>, Vec<ElementDiff>) {
    let mut diffs = Vec::new();
    diff_prop("short_name", &old.short_name, &new.short_name, &mut diffs);
    diff_prop("long_name", &old.long_name, &new.long_name, &mut diffs);
    diff_string_vec(
        "funct_classes",
        &old.funct_classes,
        &new.funct_classes,
        &mut diffs,
    );

    let mut children = Vec::new();

    let mut discard = DiffSummary::default();
    let service_diffs = compare_maps(&old.services, &new.services, compare_services, &mut discard);
    if service_diffs
        .iter()
        .any(|d| d.status != DiffStatus::Unchanged)
    {
        children.push(ElementDiff {
            name: "Services".to_owned(),
            status: DiffStatus::Modified,
            property_diffs: Vec::new(),
            children: service_diffs,
        });
    }

    let mut discard2 = DiffSummary::default();
    let job_diffs = compare_maps(
        &old.single_ecu_jobs,
        &new.single_ecu_jobs,
        compare_single_ecu_jobs,
        &mut discard2,
    );
    if job_diffs.iter().any(|d| d.status != DiffStatus::Unchanged) {
        children.push(ElementDiff {
            name: "SingleEcuJobs".to_owned(),
            status: DiffStatus::Modified,
            property_diffs: Vec::new(),
            children: job_diffs,
        });
    }

    let mut discard3 = DiffSummary::default();
    let chart_diffs = compare_maps(
        &old.state_charts,
        &new.state_charts,
        compare_state_charts,
        &mut discard3,
    );
    if chart_diffs
        .iter()
        .any(|d| d.status != DiffStatus::Unchanged)
    {
        children.push(ElementDiff {
            name: "StateCharts".to_owned(),
            status: DiffStatus::Modified,
            property_diffs: Vec::new(),
            children: chart_diffs,
        });
    }

    (diffs, children)
}

// ---------------------------------------------------------------------------
// DiagService comparison
// ---------------------------------------------------------------------------

fn compare_services(
    old: &DiagServiceSnapshot,
    new: &DiagServiceSnapshot,
) -> (Vec<PropertyDiff>, Vec<ElementDiff>) {
    let mut diffs = Vec::new();
    diff_prop("addressing", &old.addressing, &new.addressing, &mut diffs);
    diff_prop(
        "transmission_mode",
        &old.transmission_mode,
        &new.transmission_mode,
        &mut diffs,
    );
    diff_bool("is_cyclic", old.is_cyclic, new.is_cyclic, &mut diffs);
    diff_bool("is_multiple", old.is_multiple, new.is_multiple, &mut diffs);

    let mut children = Vec::new();

    // DiagComm sub-element
    let (comm_props, comm_children) = compare_diag_comms(&old.diag_comm, &new.diag_comm);
    if !comm_props.is_empty() || !comm_children.is_empty() {
        children.push(ElementDiff {
            name: "DiagComm".to_owned(),
            status: DiffStatus::Modified,
            property_diffs: comm_props,
            children: comm_children,
        });
    }

    // Request
    match (&old.request, &new.request) {
        (Some(old_req), Some(new_req)) => {
            if old_req != new_req {
                let param_diffs = compare_params_list("Request", &old_req.params, &new_req.params);
                children.extend(param_diffs);
            }
        }
        (None, Some(_)) => {
            children.push(ElementDiff {
                name: "Request".to_owned(),
                status: DiffStatus::Added,
                property_diffs: Vec::new(),
                children: Vec::new(),
            });
        }
        (Some(_), None) => {
            children.push(ElementDiff {
                name: "Request".to_owned(),
                status: DiffStatus::Removed,
                property_diffs: Vec::new(),
                children: Vec::new(),
            });
        }
        (None, None) => {}
    }

    // Positive responses
    let pos_resp_children =
        compare_response_lists("PosResponse", &old.pos_responses, &new.pos_responses);
    children.extend(pos_resp_children);

    // Negative responses
    let neg_resp_children =
        compare_response_lists("NegResponse", &old.neg_responses, &new.neg_responses);
    children.extend(neg_resp_children);

    (diffs, children)
}

// ---------------------------------------------------------------------------
// DiagComm comparison
// ---------------------------------------------------------------------------

fn compare_diag_comms(
    old: &DiagCommSnapshot,
    new: &DiagCommSnapshot,
) -> (Vec<PropertyDiff>, Vec<ElementDiff>) {
    let mut diffs = Vec::new();
    diff_prop("short_name", &old.short_name, &new.short_name, &mut diffs);
    diff_prop("long_name", &old.long_name, &new.long_name, &mut diffs);
    diff_prop("semantic", &old.semantic, &new.semantic, &mut diffs);
    diff_prop(
        "diag_class_type",
        &old.diag_class_type,
        &new.diag_class_type,
        &mut diffs,
    );
    diff_string_vec(
        "funct_classes",
        &old.funct_classes,
        &new.funct_classes,
        &mut diffs,
    );
    diff_bool(
        "is_mandatory",
        old.is_mandatory,
        new.is_mandatory,
        &mut diffs,
    );
    diff_bool(
        "is_executable",
        old.is_executable,
        new.is_executable,
        &mut diffs,
    );
    diff_bool("is_final", old.is_final, new.is_final, &mut diffs);

    let mut children = Vec::new();
    compare_audience(
        old.audience.as_ref(),
        new.audience.as_ref(),
        &mut diffs,
        &mut children,
    );

    (diffs, children)
}

/// Compare optional audience snapshots.
fn compare_audience(
    old: Option<&AudienceSnapshot>,
    new: Option<&AudienceSnapshot>,
    diffs: &mut Vec<PropertyDiff>,
    children: &mut Vec<ElementDiff>,
) {
    match (old, new) {
        (Some(old_aud), Some(new_aud)) => {
            if old_aud != new_aud {
                let mut aud_diffs = Vec::new();
                diff_bool(
                    "is_supplier",
                    old_aud.is_supplier,
                    new_aud.is_supplier,
                    &mut aud_diffs,
                );
                diff_bool(
                    "is_development",
                    old_aud.is_development,
                    new_aud.is_development,
                    &mut aud_diffs,
                );
                diff_bool(
                    "is_manufacturing",
                    old_aud.is_manufacturing,
                    new_aud.is_manufacturing,
                    &mut aud_diffs,
                );
                diff_bool(
                    "is_after_sales",
                    old_aud.is_after_sales,
                    new_aud.is_after_sales,
                    &mut aud_diffs,
                );
                diff_bool(
                    "is_after_market",
                    old_aud.is_after_market,
                    new_aud.is_after_market,
                    &mut aud_diffs,
                );
                children.push(ElementDiff {
                    name: "Audience".to_owned(),
                    status: DiffStatus::Modified,
                    property_diffs: aud_diffs,
                    children: Vec::new(),
                });
            }
        }
        (None, Some(_)) => {
            children.push(ElementDiff {
                name: "Audience".to_owned(),
                status: DiffStatus::Added,
                property_diffs: Vec::new(),
                children: Vec::new(),
            });
        }
        (Some(_), None) => {
            children.push(ElementDiff {
                name: "Audience".to_owned(),
                status: DiffStatus::Removed,
                property_diffs: Vec::new(),
                children: Vec::new(),
            });
        }
        (None, None) => {}
    }
    let _ = diffs; // audience diffs are placed in children, not top-level props
}

// ---------------------------------------------------------------------------
// Param list comparison (by short_name matching)
// ---------------------------------------------------------------------------

/// Compare two parameter lists by matching on `short_name`.
///
/// Returns element diffs wrapped in a single parent element with the given
/// `name`. If there are no differences, returns an empty `Vec`.
///
/// NOTE: Parameters are matched by `short_name`. If a request/response
/// contains multiple parameters with the same `short_name`, only the last
/// one is compared (`BTreeMap` key collision). This is acceptable because
/// duplicate `short_name` within the same scope is rare in practice.
fn compare_params_list(
    name: &str,
    old: &[ParamSnapshot],
    new: &[ParamSnapshot],
) -> Vec<ElementDiff> {
    let old_map: BTreeMap<&str, &ParamSnapshot> =
        old.iter().map(|p| (p.short_name.as_str(), p)).collect();
    let new_map: BTreeMap<&str, &ParamSnapshot> =
        new.iter().map(|p| (p.short_name.as_str(), p)).collect();

    let mut param_elements = Vec::new();

    // Removed params
    for key in old_map.keys() {
        if !new_map.contains_key(key) {
            param_elements.push(ElementDiff {
                name: (*key).to_owned(),
                status: DiffStatus::Removed,
                property_diffs: Vec::new(),
                children: Vec::new(),
            });
        }
    }

    // Modified or unchanged
    for (key, old_param) in &old_map {
        if let Some(new_param) = new_map.get(key)
            && old_param != new_param
        {
            let diffs = compare_single_param(old_param, new_param);
            param_elements.push(ElementDiff {
                name: (*key).to_owned(),
                status: DiffStatus::Modified,
                property_diffs: diffs,
                children: Vec::new(),
            });
        }
    }

    // Added params
    for key in new_map.keys() {
        if !old_map.contains_key(key) {
            param_elements.push(ElementDiff {
                name: (*key).to_owned(),
                status: DiffStatus::Added,
                property_diffs: Vec::new(),
                children: Vec::new(),
            });
        }
    }

    if param_elements.is_empty() {
        return Vec::new();
    }

    vec![ElementDiff {
        name: name.to_owned(),
        status: DiffStatus::Modified,
        property_diffs: Vec::new(),
        children: param_elements,
    }]
}

/// Compare individual fields of two `ParamSnapshot` values.
fn compare_single_param(old: &ParamSnapshot, new: &ParamSnapshot) -> Vec<PropertyDiff> {
    let mut diffs = Vec::new();
    diff_prop("short_name", &old.short_name, &new.short_name, &mut diffs);
    diff_prop("semantic", &old.semantic, &new.semantic, &mut diffs);
    diff_prop(
        "physical_default_value",
        &old.physical_default_value,
        &new.physical_default_value,
        &mut diffs,
    );
    diff_u32(
        "byte_position",
        old.byte_position,
        new.byte_position,
        &mut diffs,
    );
    diff_u32(
        "bit_position",
        old.bit_position,
        new.bit_position,
        &mut diffs,
    );
    diff_prop("param_type", &old.param_type, &new.param_type, &mut diffs);
    diff_prop(
        "specific_data_summary",
        &old.specific_data_summary,
        &new.specific_data_summary,
        &mut diffs,
    );
    diffs
}

// ---------------------------------------------------------------------------
// Response list comparison (by index)
// ---------------------------------------------------------------------------

/// Compare two response lists by index. Responses do not have a stable key,
/// so we pair them positionally.
fn compare_response_lists(
    prefix: &str,
    old: &[ResponseSnapshot],
    new: &[ResponseSnapshot],
) -> Vec<ElementDiff> {
    let mut children = Vec::new();
    let max_len = old.len().max(new.len());

    for idx in 0..max_len {
        let label = format!("{prefix}[{idx}]");
        match (old.get(idx), new.get(idx)) {
            (Some(old_resp), Some(new_resp)) => {
                if old_resp != new_resp {
                    let mut diffs = Vec::new();
                    diff_prop(
                        "response_type",
                        &old_resp.response_type,
                        &new_resp.response_type,
                        &mut diffs,
                    );
                    let param_children =
                        compare_params_list(&label, &old_resp.params, &new_resp.params);
                    // Flatten: param_children is either empty or a single wrapper element.
                    let inner_children = param_children
                        .into_iter()
                        .flat_map(|e| e.children)
                        .collect();
                    children.push(ElementDiff {
                        name: label,
                        status: DiffStatus::Modified,
                        property_diffs: diffs,
                        children: inner_children,
                    });
                }
            }
            (None, Some(_)) => {
                children.push(ElementDiff {
                    name: label,
                    status: DiffStatus::Added,
                    property_diffs: Vec::new(),
                    children: Vec::new(),
                });
            }
            (Some(_), None) => {
                children.push(ElementDiff {
                    name: label,
                    status: DiffStatus::Removed,
                    property_diffs: Vec::new(),
                    children: Vec::new(),
                });
            }
            (None, None) => {}
        }
    }

    children
}

// ---------------------------------------------------------------------------
// StateChart comparison
// ---------------------------------------------------------------------------

fn compare_state_charts(
    old: &StateChartSnapshot,
    new: &StateChartSnapshot,
) -> (Vec<PropertyDiff>, Vec<ElementDiff>) {
    let mut diffs = Vec::new();
    diff_prop("short_name", &old.short_name, &new.short_name, &mut diffs);
    diff_prop("semantic", &old.semantic, &new.semantic, &mut diffs);
    diff_prop(
        "start_state",
        &old.start_state,
        &new.start_state,
        &mut diffs,
    );
    diff_string_vec("states", &old.states, &new.states, &mut diffs);
    diff_string_vec(
        "transitions",
        &old.transitions,
        &new.transitions,
        &mut diffs,
    );

    (diffs, Vec::new())
}

// ---------------------------------------------------------------------------
// SingleEcuJob comparison
// ---------------------------------------------------------------------------

fn compare_single_ecu_jobs(
    old: &SingleEcuJobSnapshot,
    new: &SingleEcuJobSnapshot,
) -> (Vec<PropertyDiff>, Vec<ElementDiff>) {
    let mut diffs = Vec::new();
    diff_string_vec(
        "input_params",
        &old.input_params,
        &new.input_params,
        &mut diffs,
    );
    diff_string_vec(
        "output_params",
        &old.output_params,
        &new.output_params,
        &mut diffs,
    );
    diff_string_vec(
        "neg_output_params",
        &old.neg_output_params,
        &new.neg_output_params,
        &mut diffs,
    );

    let mut children = Vec::new();
    let (comm_props, comm_children) = compare_diag_comms(&old.diag_comm, &new.diag_comm);
    if !comm_props.is_empty() || !comm_children.is_empty() {
        children.push(ElementDiff {
            name: "DiagComm".to_owned(),
            status: DiffStatus::Modified,
            property_diffs: comm_props,
            children: comm_children,
        });
    }

    (diffs, children)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::snapshot::{
        DiagCommSnapshot, DiagLayerSnapshot, DiagServiceSnapshot, DtcSnapshot, EcuSnapshot,
        ParamSnapshot, RequestSnapshot, StateChartSnapshot, VariantSnapshot,
    };

    fn empty_diag_comm() -> DiagCommSnapshot {
        DiagCommSnapshot {
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

    fn empty_diag_layer() -> DiagLayerSnapshot {
        DiagLayerSnapshot {
            short_name: String::new(),
            long_name: String::new(),
            services: BTreeMap::new(),
            single_ecu_jobs: BTreeMap::new(),
            state_charts: BTreeMap::new(),
            funct_classes: Vec::new(),
        }
    }

    fn simple_ecu(name: &str, version: &str) -> EcuSnapshot {
        EcuSnapshot {
            name: name.to_owned(),
            version: version.to_owned(),
            revision: String::new(),
            metadata: Vec::new(),
            variants: BTreeMap::new(),
            functional_groups: BTreeMap::new(),
            dtcs: BTreeMap::new(),
        }
    }

    #[test]
    fn identical_snapshots_produce_no_diffs() {
        let a = simple_ecu("ECU_A", "1.0");
        let b = simple_ecu("ECU_A", "1.0");
        let result = compare(&a, &b);

        assert!(result.ecu_diffs.is_empty());
        assert!(result.variants.is_empty());
        assert!(result.functional_groups.is_empty());
        assert!(result.dtcs.is_empty());
        assert_eq!(result.summary.added, 0);
        assert_eq!(result.summary.removed, 0);
        assert_eq!(result.summary.modified, 0);
        assert_eq!(result.summary.unchanged, 0);
    }

    #[test]
    fn ecu_property_change_detected() {
        let a = simple_ecu("ECU_A", "1.0");
        let b = simple_ecu("ECU_A", "2.0");
        let result = compare(&a, &b);

        assert_eq!(result.ecu_diffs.len(), 1);
        let first = result.ecu_diffs.first();
        assert_eq!(first.map(|d| d.name.as_str()), Some("version"));
        assert_eq!(first.map(|d| d.old_value.as_str()), Some("1.0"));
        assert_eq!(first.map(|d| d.new_value.as_str()), Some("2.0"));
    }

    #[test]
    fn added_variant_counted_in_summary() {
        let a = simple_ecu("ECU", "1.0");
        let mut b = simple_ecu("ECU", "1.0");
        b.variants.insert(
            "V1".to_owned(),
            VariantSnapshot {
                is_base_variant: true,
                diag_layer: empty_diag_layer(),
            },
        );

        let result = compare(&a, &b);
        assert_eq!(result.summary.added, 1);
        assert_eq!(result.variants.len(), 1);
        assert_eq!(
            result.variants.first().map(|v| v.status),
            Some(DiffStatus::Added)
        );
    }

    #[test]
    fn removed_dtc_counted_in_summary() {
        let mut a = simple_ecu("ECU", "1.0");
        a.dtcs.insert(
            "DTC_001".to_owned(),
            DtcSnapshot {
                short_name: "DTC_001".to_owned(),
                trouble_code: 0x1234,
                display_trouble_code: "P1234".to_owned(),
                text: "Sensor fault".to_owned(),
                level: Some(1),
                is_temporary: false,
            },
        );
        let b = simple_ecu("ECU", "1.0");

        let result = compare(&a, &b);
        assert_eq!(result.summary.removed, 1);
        assert_eq!(result.dtcs.len(), 1);
        assert_eq!(
            result.dtcs.first().map(|d| d.status),
            Some(DiffStatus::Removed)
        );
    }

    #[test]
    fn modified_dtc_produces_property_diffs() {
        let dtc_old = DtcSnapshot {
            short_name: "DTC_001".to_owned(),
            trouble_code: 0x1234,
            display_trouble_code: "P1234".to_owned(),
            text: "Sensor fault".to_owned(),
            level: Some(1),
            is_temporary: false,
        };
        let dtc_new = DtcSnapshot {
            short_name: "DTC_001".to_owned(),
            trouble_code: 0x1234,
            display_trouble_code: "P1234".to_owned(),
            text: "Sensor malfunction".to_owned(),
            level: Some(2),
            is_temporary: true,
        };

        let mut a = simple_ecu("ECU", "1.0");
        a.dtcs.insert("DTC_001".to_owned(), dtc_old);
        let mut b = simple_ecu("ECU", "1.0");
        b.dtcs.insert("DTC_001".to_owned(), dtc_new);

        let result = compare(&a, &b);
        assert_eq!(result.summary.modified, 1);
        assert_eq!(result.dtcs.len(), 1);
        let first_dtc = result.dtcs.first();
        assert_eq!(first_dtc.map(|d| d.status), Some(DiffStatus::Modified));

        let prop_names: Vec<&str> = first_dtc
            .map(|d| d.property_diffs.iter().map(|p| p.name.as_str()).collect())
            .unwrap_or_default();
        assert!(prop_names.contains(&"text"));
        assert!(prop_names.contains(&"level"));
        assert!(prop_names.contains(&"is_temporary"));
        // trouble_code and display_trouble_code are unchanged
        assert!(!prop_names.contains(&"trouble_code"));
    }

    #[test]
    fn param_comparison_by_short_name() {
        let old_params = vec![
            ParamSnapshot {
                short_name: "P1".to_owned(),
                semantic: "data".to_owned(),
                physical_default_value: "0".to_owned(),
                byte_position: 0,
                bit_position: 0,
                param_type: "Value".to_owned(),
                specific_data_summary: String::new(),
            },
            ParamSnapshot {
                short_name: "P2".to_owned(),
                semantic: "data".to_owned(),
                physical_default_value: "0".to_owned(),
                byte_position: 1,
                bit_position: 0,
                param_type: "Value".to_owned(),
                specific_data_summary: String::new(),
            },
        ];
        let new_params = vec![
            ParamSnapshot {
                short_name: "P1".to_owned(),
                semantic: "data".to_owned(),
                physical_default_value: "42".to_owned(), // changed
                byte_position: 0,
                bit_position: 0,
                param_type: "Value".to_owned(),
                specific_data_summary: String::new(),
            },
            ParamSnapshot {
                short_name: "P3".to_owned(), // new param, P2 removed
                semantic: "control".to_owned(),
                physical_default_value: "0".to_owned(),
                byte_position: 2,
                bit_position: 0,
                param_type: "CodedConst".to_owned(),
                specific_data_summary: String::new(),
            },
        ];

        let result = compare_params_list("TestParams", &old_params, &new_params);
        assert_eq!(result.len(), 1); // single wrapper element
        let wrapper = result.first();
        assert_eq!(wrapper.map(|w| w.name.as_str()), Some("TestParams"));
        let children = wrapper.map(|w| &w.children);
        assert_eq!(children.map(Vec::len), Some(3)); // P2 removed, P1 modified, P3 added

        let statuses: Vec<DiffStatus> = children
            .map(|c| c.iter().map(|e| e.status).collect())
            .unwrap_or_default();
        assert!(statuses.contains(&DiffStatus::Removed));
        assert!(statuses.contains(&DiffStatus::Modified));
        assert!(statuses.contains(&DiffStatus::Added));
    }

    #[test]
    fn service_modification_detected() {
        let svc = DiagServiceSnapshot {
            diag_comm: empty_diag_comm(),
            addressing: "physical".to_owned(),
            transmission_mode: "send".to_owned(),
            is_cyclic: false,
            is_multiple: false,
            request: Some(RequestSnapshot {
                params: vec![ParamSnapshot {
                    short_name: "SID".to_owned(),
                    semantic: String::new(),
                    physical_default_value: "0x22".to_owned(),
                    byte_position: 0,
                    bit_position: 0,
                    param_type: "CodedConst".to_owned(),
                    specific_data_summary: String::new(),
                }],
            }),
            pos_responses: Vec::new(),
            neg_responses: Vec::new(),
        };

        let mut svc_new = svc.clone();
        svc_new.is_cyclic = true;

        let (props, _children) = compare_services(&svc, &svc_new);
        assert_eq!(props.len(), 1);
        assert_eq!(props.first().map(|p| p.name.as_str()), Some("is_cyclic"));
    }

    #[test]
    fn state_chart_diff_detected() {
        let old = StateChartSnapshot {
            short_name: "SC1".to_owned(),
            semantic: "lifecycle".to_owned(),
            start_state: "Init".to_owned(),
            states: vec!["Init".to_owned(), "Running".to_owned()],
            transitions: vec!["Init -> Running".to_owned()],
        };
        let new = StateChartSnapshot {
            short_name: "SC1".to_owned(),
            semantic: "lifecycle".to_owned(),
            start_state: "Init".to_owned(),
            states: vec![
                "Init".to_owned(),
                "Running".to_owned(),
                "Stopped".to_owned(),
            ],
            transitions: vec![
                "Init -> Running".to_owned(),
                "Running -> Stopped".to_owned(),
            ],
        };

        let (props, children) = compare_state_charts(&old, &new);
        assert!(children.is_empty());
        let names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"states"));
        assert!(names.contains(&"transitions"));
        assert!(!names.contains(&"short_name"));
    }
}
