// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use super::push_types_section;
use crate::tree::types::{CellType, DetailRow, DetailSectionData};

/// Build tabbed sections for EnvDataDesc DOP
pub(super) fn build_env_data_desc_dop_tabs(
    env_desc: &cda_database::datatypes::EnvDataDescDop<'_>,
    types_rows: &mut Vec<DetailRow>,
    sections: &mut Vec<DetailSectionData>,
) {
    if let Some(param_name) = env_desc.param_short_name() {
        types_rows.push(DetailRow::normal(
            vec!["Param Short Name".to_owned(), param_name.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ));
    }

    if let Some(param_path) = env_desc.param_path_short_name() {
        types_rows.push(DetailRow::normal(
            vec!["Param Path Short Name".to_owned(), param_path.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ));
    }

    if let Some(env_datas) = env_desc.env_datas() {
        types_rows.push(DetailRow::normal(
            vec!["Env Data Count".to_owned(), env_datas.len().to_string()],
            vec![CellType::Text, CellType::NumericValue],
            0,
        ));
    }

    push_types_section(std::mem::take(types_rows), sections);
}

/// Build tabbed sections for EnvData DOP
pub(super) fn build_env_data_dop_tabs(
    env_data: &cda_database::datatypes::EnvDataDop<'_>,
    types_rows: &mut Vec<DetailRow>,
    sections: &mut Vec<DetailSectionData>,
) {
    if let Some(dtc_values) = env_data.dtc_values() {
        types_rows.push(DetailRow::normal(
            vec!["DTC Values Count".to_owned(), dtc_values.len().to_string()],
            vec![CellType::Text, CellType::NumericValue],
            0,
        ));
    }

    if let Some(params) = env_data.params() {
        types_rows.push(DetailRow::normal(
            vec!["Param Count".to_owned(), params.len().to_string()],
            vec![CellType::Text, CellType::NumericValue],
            0,
        ));
    }

    push_types_section(std::mem::take(types_rows), sections);
}
