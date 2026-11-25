//! End-to-end signing test for the canonical envelope bytes.
//!
//! The protocol fixes the signing input as the RFC 8785 canonical JSON of the
//! envelope with the signature omitted. This test signs that input with the
//! device key and verifies it with the registered public key.

use core_crypto::{CryptoError, DeviceSigningKey};
use core_sync::{EntityType, OperationEnvelope, OperationType, PAYLOAD_VERSION, PROTOCOL_VERSION};
use uuid::Uuid;

fn identifier(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap()
}

fn envelope() -> OperationEnvelope {
    OperationEnvelope {
        protocol_version: PROTOCOL_VERSION,
        op_id: identifier("0199c0f0-0000-7000-8000-000000000010"),
        ledger_id: identifier("0199c0f0-0000-7000-8000-000000000001"),
        device_id: identifier("0199c0f0-0000-7000-8000-000000000002"),
        entity_type: EntityType::Transaction,
        entity_id: identifier("0199c0f0-0000-7000-8000-000000000020"),
        op_type: OperationType::Upsert,
        base_revision: 4,
        client_timestamp: 1_795_000_000_000,
        idempotency_key: identifier("0199c0f0-0000-7000-8000-000000000011"),
        key_version: 1,
        nonce: "AAECAwQFBgcICQoL".to_string(),
        ciphertext: "Ymx1ZXdoYWxlIHN5bnRoZXRpYyBjaXBoZXJ0ZXh0".to_string(),
        payload_version: PAYLOAD_VERSION,
        signature:
            "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0-Pw"
                .to_string(),
    }
}

#[test]
fn a_device_signature_verifies_the_canonical_envelope() {
    let signing_key = DeviceSigningKey::from_seed([7; 32]);
    let verifying_key = signing_key.public_key();
    let envelope = envelope();

    let message = envelope.signing_bytes();
    let signature = signing_key.sign(&message);
    verifying_key.verify(&message, &signature).unwrap();

    let mut altered = envelope.clone();
    altered.base_revision += 1;
    let altered_message = altered.signing_bytes();
    assert_ne!(altered_message, message);
    assert_eq!(
        verifying_key.verify(&altered_message, &signature).err(),
        Some(CryptoError::InvalidSignature)
    );
}

#[test]
fn the_signature_field_is_not_part_of_the_signing_input() {
    let envelope = envelope();
    let mut other_signature = envelope.clone();
    other_signature.signature.clear();
    assert_eq!(other_signature.signing_bytes(), envelope.signing_bytes());
}
