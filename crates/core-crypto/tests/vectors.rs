//! Known-answer tests for the underlying primitives.
//!
//! The vectors come from RFC 8032 (Ed25519), RFC 7748 (X25519), RFC 8439
//! (ChaCha20-Poly1305), and Argon2id outputs produced by `argon2-cffi` 25.1.0
//! against the reference implementation.

use std::time::Instant;

use core_crypto::aead;
use core_crypto::{
    Argon2idParameters, DeviceAgreementKey, DeviceAgreementPublicKey, DeviceSigningKey, Passphrase,
    SymmetricKey,
};

fn hex(text: &str) -> Vec<u8> {
    let cleaned: String = text
        .chars()
        .filter(|value| !value.is_whitespace() && *value != ':')
        .collect();
    assert_eq!(cleaned.len() % 2, 0, "odd-length hex");
    (0..cleaned.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&cleaned[index..index + 2], 16).unwrap())
        .collect()
}

fn hex_array<const N: usize>(text: &str) -> [u8; N] {
    hex(text).try_into().unwrap()
}

#[test]
fn rfc_8032_ed25519_test_one() {
    let key = DeviceSigningKey::from_seed(hex_array::<32>(
        "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
    ));
    let public = key.public_key();
    assert_eq!(
        public.to_bytes(),
        hex_array::<32>("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
    );
    let signature = key.sign(b"");
    assert_eq!(
        signature.to_bytes(),
        hex_array::<64>(
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155\
             5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
        )
    );
    public.verify(b"", &signature).unwrap();
}

#[test]
fn rfc_7748_x25519_test_vector() {
    let alice = DeviceAgreementKey::from_bytes(hex_array::<32>(
        "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a",
    ));
    let bob = DeviceAgreementKey::from_bytes(hex_array::<32>(
        "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb",
    ));
    assert_eq!(
        alice.public_key().to_bytes(),
        hex_array::<32>("8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a")
    );
    assert_eq!(
        bob.public_key().to_bytes(),
        hex_array::<32>("de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f")
    );

    let bob_public = DeviceAgreementPublicKey::from_bytes(bob.public_key().to_bytes());
    let alice_shared = alice.shared_secret(&bob_public).unwrap();
    assert_eq!(
        *alice_shared,
        hex_array::<32>("4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742")
    );

    let alice_public = DeviceAgreementPublicKey::from_bytes(alice.public_key().to_bytes());
    assert_eq!(*bob.shared_secret(&alice_public).unwrap(), *alice_shared);
}

#[test]
fn rfc_8439_chacha20_poly1305_test_vector() {
    let key = SymmetricKey::from_bytes(hex_array::<32>(
        "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f",
    ));
    let nonce = hex_array::<12>("070000004041424344454647");
    let associated_data = hex("50515253c0c1c2c3c4c5c6c7");
    let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
    let sealed = aead::seal(&key, &nonce, &associated_data, plaintext).unwrap();
    let expected = hex(
        "d31a8d34648e60db7b86afbc53ef7ec2a4aded51296e08fea9e2b5a736ee62d6\
         3dbea45e8ca9671282fafb69da92728b1a71de0a9e060b2905d6a5b67ecd3b36\
         92ddbd7f2d778b8c9803aee328091b58fab324e4fad675945585808b4831d7bc\
         3ff4def08e4b7a9de576d26586cec64b61161ae10b594f09e26a7e902ecbd0600691",
    );
    assert_eq!(sealed, expected);
    assert_eq!(
        &aead::open(&key, &nonce, &associated_data, &sealed).unwrap()[..],
        plaintext
    );
}

#[test]
fn argon2id_known_answer() {
    let parameters = Argon2idParameters::new(32, 3, 4).unwrap();
    let passphrase = Passphrase::new("bluewhale-argon2id-vector").unwrap();
    let salt = hex_array::<16>("000102030405060708090a0b0c0d0e0f");
    let key = parameters.derive_kek(&passphrase, &salt).unwrap();
    assert_eq!(
        key.expose_secret(),
        &hex_array::<32>("21a9e39290a9262709a855d90262c5d2760aebab20eb2f0923db8de52978e902")
    );
}

#[test]
#[ignore = "records the version 1 Argon2id answer and timing; run in release with --ignored --nocapture"]
fn argon2id_version_one_known_answer_and_timing() {
    let parameters = Argon2idParameters::V1;
    let passphrase = Passphrase::new("bluewhale-argon2id-vector").unwrap();
    let salt = hex_array::<16>("000102030405060708090a0b0c0d0e0f");
    let start = Instant::now();
    let key = parameters.derive_kek(&passphrase, &salt).unwrap();
    let elapsed = start.elapsed();
    assert_eq!(
        key.expose_secret(),
        &hex_array::<32>("7f8f83cf98493030d3f182a1814be38b47939d7e3c06bb61bc17f709f3106122")
    );
    eprintln!("argon2id m=65536 t=3 p=1: {elapsed:?}");
}
