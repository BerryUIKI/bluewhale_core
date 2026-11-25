//! Conformance tests for the sync protocol v1 contract.
//!
//! Every fixture is validated against the schema at
//! `contracts/sync/v1/sync.schema.json`, its SHA-256 is checked against the
//! manifest, and every valid document round-trips through the Rust types.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use core_sync::{
    AckRequest, ApiError, AuthSession, Device, DeviceList, DeviceRegistrationRequest,
    LedgerKeyUpdateRequest, LedgerList, LedgerRegistrationRequest, LedgerSummary, MagicLinkRequest,
    OperationEnvelope, PROTOCOL_VERSION, PullResponse, PushRequest, PushResponse, RefreshRequest,
    Snapshot, SnapshotUploadRequest, SnapshotUploadResponse, VerifyRequest,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};

const CONTRACT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../contracts/sync/v1");

const REQUIRED_DOCUMENTS: [&str; 20] = [
    "operationEnvelope",
    "pushRequest",
    "pushResponse",
    "pullResponse",
    "ackRequest",
    "error",
    "magicLinkRequest",
    "verifyRequest",
    "authSession",
    "refreshRequest",
    "deviceRegistrationRequest",
    "device",
    "deviceList",
    "ledgerRegistrationRequest",
    "ledgerSummary",
    "ledgerList",
    "ledgerKeyUpdateRequest",
    "snapshotUploadRequest",
    "snapshotUploadResponse",
    "snapshot",
];

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    manifest_version: u32,
    protocol_version: u32,
    schema: String,
    cases: Vec<Case>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    file: String,
    document: String,
    valid: bool,
    sha256: String,
}

fn contract_path(relative: &str) -> PathBuf {
    Path::new(CONTRACT_DIR).join(relative)
}

fn load_schema() -> Value {
    let text = fs::read_to_string(contract_path("sync.schema.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn load_manifest() -> Manifest {
    let text = fs::read_to_string(contract_path("fixtures/manifest.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn load_case(case: &Case) -> Value {
    let text = fs::read_to_string(contract_path(&format!("fixtures/{}", case.file))).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn validator(schema: &Value, document: &str) -> jsonschema::Validator {
    assert!(
        schema["$defs"].get(document).is_some(),
        "missing $defs.{document}"
    );
    let wrapper = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$ref": format!("#/$defs/{document}"),
        "$defs": schema["$defs"].clone(),
    });
    jsonschema::validator_for(&wrapper).unwrap()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        write!(&mut hex, "{byte:02x}").unwrap();
    }
    hex
}

fn assert_round_trip<T: DeserializeOwned + Serialize>(value: &Value, file: &str) {
    let typed: T = serde_json::from_value(value.clone())
        .unwrap_or_else(|error| panic!("{file} does not deserialize: {error}"));
    let encoded = serde_json::to_value(&typed).unwrap();
    assert_eq!(&encoded, value, "{file} does not round-trip");
}

fn round_trip(document: &str, value: &Value, file: &str) {
    match document {
        "operationEnvelope" => assert_round_trip::<OperationEnvelope>(value, file),
        "pushRequest" => assert_round_trip::<PushRequest>(value, file),
        "pushResponse" => assert_round_trip::<PushResponse>(value, file),
        "pullResponse" => assert_round_trip::<PullResponse>(value, file),
        "ackRequest" => assert_round_trip::<AckRequest>(value, file),
        "error" => assert_round_trip::<ApiError>(value, file),
        "magicLinkRequest" => assert_round_trip::<MagicLinkRequest>(value, file),
        "verifyRequest" => assert_round_trip::<VerifyRequest>(value, file),
        "authSession" => assert_round_trip::<AuthSession>(value, file),
        "refreshRequest" => assert_round_trip::<RefreshRequest>(value, file),
        "deviceRegistrationRequest" => {
            assert_round_trip::<DeviceRegistrationRequest>(value, file);
        }
        "device" => assert_round_trip::<Device>(value, file),
        "deviceList" => assert_round_trip::<DeviceList>(value, file),
        "ledgerRegistrationRequest" => {
            assert_round_trip::<LedgerRegistrationRequest>(value, file);
        }
        "ledgerSummary" => assert_round_trip::<LedgerSummary>(value, file),
        "ledgerList" => assert_round_trip::<LedgerList>(value, file),
        "ledgerKeyUpdateRequest" => {
            assert_round_trip::<LedgerKeyUpdateRequest>(value, file);
        }
        "snapshotUploadRequest" => assert_round_trip::<SnapshotUploadRequest>(value, file),
        "snapshotUploadResponse" => assert_round_trip::<SnapshotUploadResponse>(value, file),
        "snapshot" => assert_round_trip::<Snapshot>(value, file),
        other => panic!("no Rust type registered for {other}"),
    }
}

#[test]
fn manifest_cases_match_the_fixture_directory() {
    let manifest = load_manifest();
    assert_eq!(manifest.manifest_version, 1);
    assert_eq!(manifest.protocol_version, PROTOCOL_VERSION);
    assert_eq!(manifest.schema, "sync.schema.json");

    let listed: BTreeSet<&str> = manifest
        .cases
        .iter()
        .map(|case| case.file.as_str())
        .collect();
    assert_eq!(
        listed.len(),
        manifest.cases.len(),
        "duplicate manifest case"
    );

    let mut on_disk = BTreeSet::new();
    for directory in ["valid", "invalid"] {
        for entry in fs::read_dir(contract_path(&format!("fixtures/{directory}"))).unwrap() {
            let name = entry.unwrap().file_name().into_string().unwrap();
            if name.ends_with(".json") {
                on_disk.insert(format!("{directory}/{name}"));
            }
        }
    }
    let on_disk: BTreeSet<&str> = on_disk.iter().map(String::as_str).collect();
    assert_eq!(listed, on_disk, "manifest and fixture files disagree");
}

#[test]
fn every_case_matches_its_expected_schema_result() {
    let schema = load_schema();
    let manifest = load_manifest();
    for case in &manifest.cases {
        let bytes = fs::read(contract_path(&format!("fixtures/{}", case.file))).unwrap();
        assert_eq!(
            sha256_hex(&bytes),
            case.sha256,
            "hash mismatch for {}",
            case.file
        );
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let result = validator(&schema, &case.document).validate(&value);
        assert_eq!(
            result.is_ok(),
            case.valid,
            "{} expected valid = {}",
            case.file,
            case.valid
        );
    }
}

#[test]
fn every_valid_case_round_trips_through_the_rust_types() {
    let manifest = load_manifest();
    for case in manifest.cases.iter().filter(|case| case.valid) {
        let value = load_case(case);
        round_trip(&case.document, &value, &case.file);
    }
}

#[test]
fn every_operation_envelope_case_is_enforced_in_rust() {
    let manifest = load_manifest();
    for case in manifest
        .cases
        .iter()
        .filter(|case| case.document == "operationEnvelope")
    {
        let value = load_case(case);
        let parsed = serde_json::from_value::<OperationEnvelope>(value);
        if case.valid {
            parsed
                .unwrap_or_else(|error| panic!("{} should deserialize: {error}", case.file))
                .validate()
                .unwrap_or_else(|error| panic!("{} should validate: {error}", case.file));
        } else {
            let rejected = match parsed {
                Ok(envelope) => envelope.validate().is_err(),
                Err(_) => true,
            };
            assert!(rejected, "{} should be rejected in Rust", case.file);
        }
    }
}

#[test]
fn every_top_level_document_has_a_valid_case() {
    let manifest = load_manifest();
    let schema = load_schema();
    for document in REQUIRED_DOCUMENTS {
        assert!(
            schema["$defs"].get(document).is_some(),
            "missing $defs.{document}"
        );
        assert!(
            manifest
                .cases
                .iter()
                .any(|case| case.document == document && case.valid),
            "no valid fixture for {document}"
        );
    }
}

#[test]
fn oversized_batches_are_rejected_by_the_schema() {
    let schema = load_schema();
    let value = load_case(&Case {
        file: "valid/operation-envelope.json".to_string(),
        document: "operationEnvelope".to_string(),
        valid: true,
        sha256: String::new(),
    });
    let ops: Vec<Value> = std::iter::repeat_n(value, 501).collect();
    let request = serde_json::json!({ "ops": ops });
    assert!(
        validator(&schema, "pushRequest")
            .validate(&request)
            .is_err()
    );
}
