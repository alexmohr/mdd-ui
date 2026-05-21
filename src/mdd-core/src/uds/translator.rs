/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use anyhow::{Context, Result, bail};
use cda_core::DiagServiceResponseStruct;
use cda_interfaces::{
    DiagComm, DiagCommType, DiagServiceError, DynamicPlugin, EcuManager as EcuManagerTrait,
    EcuManagerType, EcuSchemaProvider, FunctionalDescriptionConfig, HashMap, HashMapExtensions,
    Protocol,
    datatypes::{
        ComParams, DatabaseNamingConvention, DiagnosticServiceAffixPosition, DtcField, DtcRecord,
    },
    diagservices::{
        DiagServiceJsonResponse, DiagServiceResponse, DiagServiceResponseType, MappedNRC,
        UdsPayloadData,
    },
    service_ids,
};
use cda_plugin_security::DefaultSecurityPluginData;
use serde::{Deserialize, Serialize};

type CdaEcuManager = cda_core::EcuManager<DefaultSecurityPluginData>;

// ── Fake response for variant detection ─────────────────────────────────
// `evaluate_variant` calls `into_json()` and then compares JSON field values
// against expected matching-parameter values.  This lightweight type lets us
// hand-craft the exact JSON the matcher needs without building real UDS
// byte payloads.

#[derive(Clone)]
struct FakeVariantResponse {
    name: String,
    json: serde_json::Value,
}

impl DiagServiceResponse for FakeVariantResponse {
    fn is_empty(&self) -> bool {
        false
    }
    fn service_name(&self) -> String {
        self.name.clone()
    }
    fn response_type(&self) -> DiagServiceResponseType {
        DiagServiceResponseType::Positive
    }
    fn get_raw(&self) -> &[u8] {
        &[]
    }
    fn into_json(self) -> Result<DiagServiceJsonResponse, DiagServiceError> {
        Ok(DiagServiceJsonResponse {
            data: self.json,
            errors: Vec::new(),
        })
    }
    fn as_nrc(&self) -> Result<MappedNRC, DiagServiceError> {
        Ok(MappedNRC {
            code: None,
            description: None,
            sid: None,
        })
    }
    fn get_dtcs(&self) -> Result<Vec<(DtcField, DtcRecord)>, DiagServiceError> {
        Ok(Vec::new())
    }
}

/// Wrapper around the CDA `EcuManager` for UDS translation and encoding.
pub struct UdsTranslator {
    manager: CdaEcuManager,
    path: String,
}

/// A service matched by name or SID lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedService {
    pub name: String,
    pub service_type: String,
}

/// A variant available in the loaded MDD database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantInfo {
    pub name: String,
    pub is_base_variant: bool,
    pub is_active: bool,
}

/// Result of translating UDS bytes to a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdsLookupResult {
    pub matched_services: Vec<MatchedService>,
    pub sid_name: String,
}

/// Result of encoding JSON parameters into UDS bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdsEncodeResult {
    pub service_name: String,
    pub hex_bytes: String,
    pub raw_bytes: Vec<u8>,
}

/// JSON Schema description for a service's request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSchemaResult {
    pub service_name: String,
    pub request_schema: Option<serde_json::Value>,
    pub response_schema: Option<serde_json::Value>,
}

impl UdsTranslator {
    /// Create a new translator from an MDD database file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be loaded or the `EcuManager`
    /// cannot be initialised.
    pub async fn new(path: &str) -> Result<Self> {
        let db = crate::database::load_mdd_ignore_protocol(path)
            .context("Failed to load MDD database for UDS translator")?;

        let func_config = FunctionalDescriptionConfig {
            description_database: String::new(),
            enabled_functional_groups: None,
            protocol_position: DiagnosticServiceAffixPosition::Suffix,
        };

        let com_params = ComParams::default();

        let mut manager = CdaEcuManager::new(
            db,
            Protocol::default_doip(),
            &com_params,
            DatabaseNamingConvention::default(),
            EcuManagerType::Ecu,
            &func_config,
            true,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("Failed to create CDA EcuManager")?;

        // Trigger variant detection with dummy responses so the EcuManager
        // falls back to the base variant (we have no live UDS link).
        let dummy_responses: HashMap<String, DiagServiceResponseStruct> = manager
            .get_variant_detection_requests()
            .keys()
            .map(|name| {
                let resp = DiagServiceResponseStruct {
                    service: DiagComm::new(name.clone(), DiagCommType::Data),
                    data: vec![0x7F, 0x00, 0x10], // generic NRC
                    mapped_data: None,
                    response_type: DiagServiceResponseType::Negative,
                };
                (name.clone(), resp)
            })
            .collect();

        let _ = manager.detect_variant(dummy_responses).await;

        Ok(Self {
            manager,
            path: path.to_owned(),
        })
    }

    /// Look up which diagnostic service(s) match the given raw UDS bytes.
    ///
    /// The first byte is used as the Service Identifier (SID).  All
    /// services registered under that SID are returned (partial match).
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes slice is empty.
    pub fn lookup_service(&self, bytes: &[u8]) -> Result<UdsLookupResult> {
        let Some(&sid) = bytes.first() else {
            bail!("UDS bytes must not be empty");
        };
        let sid_name = describe_sid(sid);

        let result = self.manager.lookup_diagcomms_by_request_prefix(bytes);
        let matched: Vec<MatchedService> = result
            .unwrap_or_default()
            .into_iter()
            .map(|dc| matched_service_from_diagcomm(&dc))
            .collect();

        Ok(UdsLookupResult {
            matched_services: matched,
            sid_name,
        })
    }

    /// Encode a JSON parameter map into raw UDS request bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the service is not found or the JSON cannot be
    /// encoded into UDS bytes.
    pub async fn uds_encode(
        &self,
        service_name: &str,
        json: &serde_json::Value,
    ) -> Result<UdsEncodeResult> {
        let diag_comm = self.find_service(service_name)?;
        let security_plugin: cda_interfaces::DynamicPlugin = Box::new(());

        // Pre-map numeric values to TextTable labels (e.g. 1 → "true") so
        // the CDA receives correct text entries on the first attempt.
        let mapped = self.map_text_table_values(service_name, json).await;

        let uds_data = UdsPayloadData::ParameterMap(
            mapped
                .as_object()
                .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default(),
        );

        let payload = self
            .manager
            .create_uds_payload(&diag_comm, &security_plugin, Some(uds_data), None)
            .await;

        let payload = match payload {
            Ok(p) => p,
            Err(e) if is_unsupported_dop_error(&e) => {
                if json.as_object().is_some_and(|obj| !obj.is_empty()) {
                    return Ok(UdsEncodeResult {
                        service_name: service_name.to_owned(),
                        hex_bytes: String::new(),
                        raw_bytes: Vec::new(),
                    });
                }
                match self
                    .manager
                    .create_uds_payload(&diag_comm, &security_plugin, None, None)
                    .await
                {
                    Ok(p) => p,
                    Err(_) => {
                        return Ok(UdsEncodeResult {
                            service_name: service_name.to_owned(),
                            hex_bytes: String::new(),
                            raw_bytes: Vec::new(),
                        });
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("{e}"))
                    .context("Failed to create UDS payload from JSON");
            }
        };

        let hex_bytes = payload
            .data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        Ok(UdsEncodeResult {
            service_name: service_name.to_owned(),
            hex_bytes,
            raw_bytes: payload.data,
        })
    }

    /// List all available diagnostic services in the loaded MDD.
    ///
    /// Enumerates services across every known UDS SID, supplemented by the
    /// component APIs so that no service is missed regardless of how the
    /// database exposes it.
    #[must_use]
    pub fn list_services(&self) -> Vec<MatchedService> {
        let security_plugin: DynamicPlugin = Box::new(());
        let mut seen = std::collections::HashSet::new();
        let mut services = Vec::new();

        // 1. Walk all known SIDs via prefix-based lookup.
        for &sid in ALL_SIDS {
            let result = self.manager.lookup_diagcomms_by_request_prefix(&[sid]);
            for dc in result.unwrap_or_default() {
                if seen.insert(dc.name.clone()) {
                    services.push(matched_service_from_diagcomm(&dc));
                }
            }
        }

        // 2. Supplement with component APIs.
        let data_info = self.manager.get_components_data_info(&security_plugin);
        for info in data_info {
            if seen.insert(info.name.clone()) {
                services.push(MatchedService {
                    name: info.name,
                    service_type: "data".to_owned(),
                });
            }
        }
        if let Ok(configs) = self
            .manager
            .get_components_configurations_info(&security_plugin)
        {
            for info in configs {
                if seen.insert(info.name.clone()) {
                    services.push(MatchedService {
                        name: info.name,
                        service_type: "configurations".to_owned(),
                    });
                }
            }
        }
        for info in self.manager.get_components_single_ecu_jobs_info() {
            if seen.insert(info.name.clone()) {
                services.push(MatchedService {
                    name: info.name,
                    service_type: "operations".to_owned(),
                });
            }
        }

        services
    }

    /// Return the JSON Schema for a service's request and response
    /// parameters.
    ///
    /// The schemas expose which fields are coded constants (`const`) and
    /// which are user-editable, enabling a tailored UI.
    ///
    /// When the service cannot be resolved to a `DiagComm` (e.g. it is only
    /// known through the component APIs), schemas will be `None`.
    pub async fn service_schema(&self, service_name: &str) -> ServiceSchemaResult {
        let diag_comm = self.find_service(service_name).ok();

        let request_schema = match &diag_comm {
            Some(dc) => self
                .manager
                .schema_for_request(dc)
                .await
                .ok()
                .and_then(|s| {
                    s.into_schema()
                        .and_then(|schema| serde_json::to_value(schema).ok())
                }),
            None => None,
        };

        let response_schema = match &diag_comm {
            Some(dc) => self
                .manager
                .schema_for_responses(dc)
                .await
                .ok()
                .and_then(|s| {
                    s.into_schema()
                        .and_then(|schema| serde_json::to_value(schema).ok())
                }),
            None => None,
        };

        ServiceSchemaResult {
            service_name: service_name.to_owned(),
            request_schema,
            response_schema,
        }
    }

    /// List all variants in the loaded MDD database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be reloaded for pattern
    /// extraction.
    pub fn list_variants(&self) -> Result<Vec<VariantInfo>> {
        let db = crate::database::load_mdd(&self.path)
            .context("Failed to reload MDD for variant listing")?;

        let active_name = self.manager.variant().name;

        // `EcuData::variants` lives in a private module so we cannot use
        // a method reference here.
        #[allow(clippy::redundant_closure_for_method_calls)]
        let Some(variants) = db.ecu_data().ok().and_then(|e| e.variants()) else {
            return Ok(Vec::new());
        };

        Ok(variants
            .iter()
            .filter_map(|v| {
                let name = v.diag_layer()?.short_name()?.to_owned();
                let is_active = active_name.as_deref().is_some_and(|an| an == name);
                Some(VariantInfo {
                    name,
                    is_base_variant: v.is_base_variant(),
                    is_active,
                })
            })
            .collect())
    }

    /// Switch the active variant by crafting fake detection responses that
    /// match exactly the target variant's matching-parameter patterns.
    ///
    /// # Errors
    ///
    /// Returns an error if the variant name is not found or detection fails.
    pub async fn select_variant(&mut self, variant_name: &str) -> Result<VariantInfo> {
        let db = crate::database::load_mdd(&self.path)
            .context("Failed to reload MDD for variant selection")?;

        let variants = db
            .ecu_data()
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .variants()
            .context("ECU has no variants")?;

        let target = variants
            .iter()
            .find(|v| {
                v.diag_layer()
                    .and_then(|dl| dl.short_name())
                    .is_some_and(|n| n == variant_name)
            })
            .with_context(|| format!("Variant '{variant_name}' not found in database"))?;

        // Build fake responses: for each matching parameter in the first
        // matching pattern, create a JSON object with { param_name: expected_value }.
        let mut responses: HashMap<String, FakeVariantResponse> = HashMap::new();

        if let Some(patterns) = target.variant_pattern()
            && let Some(pattern) = patterns.iter().next()
            && let Some(params) = pattern.matching_parameter()
        {
            for mp in &params {
                let Some(service_name) = mp
                    .diag_service()
                    .and_then(|ds| ds.diag_comm())
                    .and_then(|dc| dc.short_name())
                else {
                    continue;
                };
                let param_name = mp
                    .out_param()
                    .and_then(|op| op.short_name())
                    .unwrap_or_default();
                let expected_value = mp.expected_value().unwrap_or_default();

                let entry = responses.entry(service_name.to_owned()).or_insert_with(|| {
                    FakeVariantResponse {
                        name: service_name.to_owned(),
                        json: serde_json::json!({}),
                    }
                });

                if let Some(obj) = entry.json.as_object_mut() {
                    obj.insert(
                        param_name.to_owned(),
                        serde_json::Value::String(expected_value.to_owned()),
                    );
                }
            }
        }

        if responses.is_empty() {
            bail!(
                "Variant '{variant_name}' has no matching parameters; cannot fake detection \
                 responses"
            );
        }

        self.manager
            .detect_variant(responses)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("Variant detection failed with crafted responses")?;

        let v = self.manager.variant();

        Ok(VariantInfo {
            name: v.name.unwrap_or_default(),
            is_base_variant: v.is_base_variant,
            is_active: true,
        })
    }

    /// Ensure the translator's active variant matches the given name.
    ///
    /// If the currently active variant already matches `variant_name` this is
    /// a no-op.  Otherwise, [`select_variant`](Self::select_variant) is called
    /// to switch.
    ///
    /// # Errors
    ///
    /// Returns an error if variant switching fails.
    pub async fn ensure_variant(&mut self, variant_name: &str) -> Result<()> {
        let active = self.manager.variant().name;
        if active.as_deref().is_some_and(|n| n == variant_name) {
            return Ok(());
        }
        self.select_variant(variant_name).await?;
        Ok(())
    }

    /// Find a `DiagComm` by its short name, searching across all known SIDs.
    ///
    /// Tries the CDA `lookup_service_by_sid_and_name` first (which compares
    /// against trimmed names without protocol affixes).  If that fails, falls
    /// back to a prefix-based scan and matches by the original ODX
    /// `lookup_name` so that full names (e.g. with `_DoIP` suffix) also
    /// resolve.
    fn find_service(&self, name: &str) -> Result<DiagComm> {
        for &sid in ALL_SIDS {
            if let Ok(dc) = self.manager.lookup_service_by_sid_and_name(sid, name, None) {
                return Ok(dc);
            }
        }

        // Fallback: match against the original ODX lookup_name (includes
        // protocol affixes that the CDA trimmed-name lookup above strips).
        for &sid in ALL_SIDS {
            let Ok(services) = self.manager.lookup_diagcomms_by_request_prefix(&[sid]) else {
                continue;
            };
            if let Some(dc) = services.into_iter().find(|dc| {
                dc.lookup_name
                    .as_deref()
                    .is_some_and(|ln| ln.eq_ignore_ascii_case(name))
                    || dc.name.eq_ignore_ascii_case(name)
            }) {
                return Ok(dc);
            }
        }

        bail!("Service '{name}' not found in the database")
    }

    /// Map numeric parameter values to `TextTable` enum labels using the
    /// service schema.  For each param with an `enum` constraint in the
    /// schema, the numeric value is used as an index into the enum array.
    async fn map_text_table_values(
        &self,
        service_name: &str,
        json: &serde_json::Value,
    ) -> serde_json::Value {
        let Some(obj) = json.as_object() else {
            return json.clone();
        };

        let schema_result = self.service_schema(service_name).await;
        let Some(schema) = schema_result.request_schema else {
            return json.clone();
        };

        let properties = schema.get("properties").and_then(|p| p.as_object());

        let mut mapped = obj.clone();
        for (key, val) in obj {
            let enum_values = properties
                .and_then(|props| props.get(key))
                .and_then(|prop| prop.get("enum"))
                .and_then(|e| e.as_array());

            let Some(entries) = enum_values else {
                continue;
            };

            let idx = match val {
                serde_json::Value::String(s) => s.parse::<usize>().ok(),
                serde_json::Value::Number(n) => n.as_u64().and_then(|v| usize::try_from(v).ok()),
                _ => None,
            };

            if let Some(i) = idx
                && let Some(entry) = entries.get(i)
            {
                mapped.insert(key.clone(), entry.clone());
            }
        }

        serde_json::Value::Object(mapped)
    }
}

/// Returns `true` when a `DiagServiceError` indicates the service uses
/// `EnvData`, `EnvDataDesc`, DTC, `StaticField`, or `DynamicLengthField`
/// DOPs that cannot be mapped to a UDS request payload through the normal
/// parameter path.
fn is_unsupported_dop_error(e: &DiagServiceError) -> bool {
    let msg = format!("{e}");
    msg.contains("EnvData")
        || msg.contains("DTC DoPs")
        || msg.contains("StaticField")
        || msg.contains("DynamicLengthField")
}

/// All UDS SIDs we scan when enumerating services.
const ALL_SIDS: &[u8] = &[
    service_ids::READ_DATA_BY_IDENTIFIER,
    service_ids::WRITE_DATA_BY_IDENTIFIER,
    service_ids::ROUTINE_CONTROL,
    service_ids::INPUT_OUTPUT_CONTROL_BY_IDENTIFIER,
    service_ids::SESSION_CONTROL,
    service_ids::ECU_RESET,
    service_ids::CLEAR_DIAGNOSTIC_INFORMATION,
    service_ids::READ_DTC_INFORMATION,
    service_ids::SECURITY_ACCESS,
    service_ids::AUTHENTICATION,
    service_ids::COMMUNICATION_CONTROL,
    service_ids::CONTROL_DTC_SETTING,
    service_ids::TESTER_PRESENT,
    service_ids::REQUEST_DOWNLOAD,
    service_ids::TRANSFER_DATA,
    service_ids::REQUEST_TRANSFER_EXIT,
];

/// Build a `MatchedService` from a CDA `DiagComm`.
fn matched_service_from_diagcomm(dc: &DiagComm) -> MatchedService {
    let category = match dc.type_ {
        DiagCommType::Data => "data",
        DiagCommType::Configurations => "configurations",
        DiagCommType::Operations => "operations",
        DiagCommType::Modes => "modes",
        DiagCommType::Faults => "faults",
    };
    MatchedService {
        name: dc.name.clone(),
        service_type: category.to_owned(),
    }
}

/// Return a human-readable name for a UDS Service Identifier.
fn describe_sid(sid: u8) -> String {
    match sid {
        service_ids::SESSION_CONTROL => "DiagnosticSessionControl".to_owned(),
        service_ids::ECU_RESET => "ECUReset".to_owned(),
        service_ids::CLEAR_DIAGNOSTIC_INFORMATION => "ClearDiagnosticInformation".to_owned(),
        service_ids::READ_DTC_INFORMATION => "ReadDTCInformation".to_owned(),
        service_ids::READ_DATA_BY_IDENTIFIER => "ReadDataByIdentifier".to_owned(),
        service_ids::SECURITY_ACCESS => "SecurityAccess".to_owned(),
        service_ids::COMMUNICATION_CONTROL => "CommunicationControl".to_owned(),
        service_ids::AUTHENTICATION => "Authentication".to_owned(),
        service_ids::WRITE_DATA_BY_IDENTIFIER => "WriteDataByIdentifier".to_owned(),
        service_ids::INPUT_OUTPUT_CONTROL_BY_IDENTIFIER => {
            "InputOutputControlByIdentifier".to_owned()
        }
        service_ids::ROUTINE_CONTROL => "RoutineControl".to_owned(),
        service_ids::REQUEST_DOWNLOAD => "RequestDownload".to_owned(),
        service_ids::TRANSFER_DATA => "TransferData".to_owned(),
        service_ids::REQUEST_TRANSFER_EXIT => "RequestTransferExit".to_owned(),
        service_ids::TESTER_PRESENT => "TesterPresent".to_owned(),
        service_ids::CONTROL_DTC_SETTING => "ControlDTCSetting".to_owned(),
        service_ids::NEGATIVE_RESPONSE => "NegativeResponse".to_owned(),
        _ if sid >= 0x40 => {
            let request_sid = sid.saturating_sub(0x40);
            let req_name = describe_sid(request_sid);
            format!("{req_name} (positive response)")
        }
        _ => format!("Unknown SID 0x{sid:02X}"),
    }
}

/// Parse a hex string like `"22 F1 90"` or `"22F190"` into raw bytes.
///
/// # Errors
///
/// Returns an error if the input contains invalid hex characters.
pub fn parse_hex_string(hex: &str) -> Result<Vec<u8>> {
    let clean: String = hex.chars().filter(char::is_ascii_hexdigit).collect();

    if clean.len().checked_rem(2) != Some(0) {
        bail!(
            "Hex string has odd number of digits ({} digits)",
            clean.len()
        );
    }

    let mut bytes = Vec::with_capacity(clean.len().checked_div(2).unwrap_or(0));
    let mut chars = clean.chars();
    while let (Some(hi), Some(lo)) = (chars.next(), chars.next()) {
        let byte = u8::from_str_radix(&format!("{hi}{lo}"), 16).context("Invalid hex digit")?;
        bytes.push(byte);
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_spaced() {
        let bytes = parse_hex_string("22 F1 90").unwrap();
        assert_eq!(bytes, vec![0x22, 0xF1, 0x90]);
    }

    #[test]
    fn parse_hex_compact() {
        let bytes = parse_hex_string("22F190").unwrap();
        assert_eq!(bytes, vec![0x22, 0xF1, 0x90]);
    }

    #[test]
    fn parse_hex_odd_digits_fails() {
        assert!(parse_hex_string("22F").is_err());
    }

    #[test]
    fn describe_sid_known() {
        assert_eq!(describe_sid(0x22), "ReadDataByIdentifier");
        assert_eq!(describe_sid(0x10), "DiagnosticSessionControl");
    }

    #[test]
    fn describe_sid_response() {
        assert_eq!(
            describe_sid(0x62),
            "ReadDataByIdentifier (positive response)"
        );
    }

    #[test]
    fn describe_sid_unknown() {
        assert_eq!(describe_sid(0xAA), "Unknown SID 0xAA");
    }
}
