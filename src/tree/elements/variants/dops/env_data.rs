/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::push_types_section;
use crate::tree::types::{CellType, DetailCell, DetailRow, DetailSectionData};

/// Build tabbed sections for `EnvDataDesc` DOP
pub(super) fn build_env_data_desc_dop_tabs(
    env_desc: &cda_database::datatypes::EnvDataDescDop<'_>,
    types_rows: &mut Vec<DetailRow>,
    sections: &mut Vec<DetailSectionData>,
) {
    if let Some(param_name) = env_desc.param_short_name() {
        types_rows.push(DetailRow::normal(
            vec![DetailCell::text("Param Short Name"), DetailCell::text(param_name)],
            0,
        ));
    }

    if let Some(param_path) = env_desc.param_path_short_name() {
        types_rows.push(DetailRow::normal(
            vec![DetailCell::text("Param Path Short Name"), DetailCell::text(param_path)],
            0,
        ));
    }

    if let Some(env_datas) = env_desc.env_datas() {
        types_rows.push(DetailRow::normal(
            vec![DetailCell::text("Env Data Count"), DetailCell::new(env_datas.len().to_string(), CellType::NumericValue)],
            0,
        ));
    }

    push_types_section(std::mem::take(types_rows), sections);
}

/// Build tabbed sections for `EnvData` DOP
pub(super) fn build_env_data_dop_tabs(
    env_data: &cda_database::datatypes::EnvDataDop<'_>,
    types_rows: &mut Vec<DetailRow>,
    sections: &mut Vec<DetailSectionData>,
) {
    if let Some(dtc_values) = env_data.dtc_values() {
        types_rows.push(DetailRow::normal(
            vec![DetailCell::text("DTC Values Count"), DetailCell::new(dtc_values.len().to_string(), CellType::NumericValue)],
            0,
        ));
    }

    if let Some(params) = env_data.params() {
        types_rows.push(DetailRow::normal(
            vec![DetailCell::text("Param Count"), DetailCell::new(params.len().to_string(), CellType::NumericValue)],
            0,
        ));
    }

    push_types_section(std::mem::take(types_rows), sections);
}
