/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

mod details;
mod diag_comms;

use cda_database::datatypes::{ParamType, Parameter};
pub(crate) use details::build_service_overview_section;
pub use diag_comms::{add_diag_comms, get_parent_ref_services_recursive};

/// Extract the hex-formatted coded value from a `CodedConst` parameter.
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
                if num <= 0xFF {
                    format!("0x{num:02X}")
                } else if num <= 0xFFFF {
                    format!("0x{num:04X}")
                } else if num <= 0x00FF_FFFF {
                    format!("0x{num:06X}")
                } else if num <= 0xFFFF_FFFF {
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

/// Extract the DOP short name from a `Value`-type parameter.
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
        .map(std::borrow::ToOwned::to_owned)
        .unwrap_or_default()
}

/// Extract a type badge string for the DOP referenced by a `Value`-type parameter.
/// Returns an empty string if no DOP is found or the type cannot be determined.
pub fn extract_dop_badge(param: &Parameter<'_>) -> &'static str {
    use cda_database::datatypes::DataOperationVariant;

    if !matches!(param.param_type(), Ok(ParamType::Value)) {
        return "";
    }

    let Some(raw_dop) = param.specific_data_as_value().and_then(|vd| vd.dop()) else {
        return "";
    };
    let dop = cda_database::datatypes::DataOperation(raw_dop);

    match dop.variant() {
        Ok(DataOperationVariant::Normal(_)) => "[DOP] ",
        Ok(DataOperationVariant::Dtc(_)) => "[DTC] ",
        Ok(DataOperationVariant::Structure(_)) => "[Struct] ",
        Ok(DataOperationVariant::StaticField(_)) => "[SField] ",
        Ok(DataOperationVariant::DynamicLengthField(_)) => "[DynLen] ",
        Ok(DataOperationVariant::EndOfPdu(_)) => "[EoPdu] ",
        Ok(DataOperationVariant::Mux(_)) => "[Mux] ",
        Ok(DataOperationVariant::EnvData(_)) => "[EnvData] ",
        Ok(DataOperationVariant::EnvDataDesc(_)) => "[EnvDesc] ",
        _ => "",
    }
}
