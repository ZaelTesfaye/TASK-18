use silverscreen_backend::services::encryption_service;
use std::collections::HashMap;

fn test_key() -> Vec<u8> {
    b"0123456789abcdef0123456789abcdef".to_vec() // 32 bytes
}

fn test_keys() -> HashMap<u32, Vec<u8>> {
    let mut m = HashMap::new();
    m.insert(1, test_key());
    m
}

// ---------------------------------------------------------------------------
// Encrypt / Decrypt roundtrip
// ---------------------------------------------------------------------------

#[test]
fn test_encrypt_decrypt_roundtrip() {
    let plaintext = "Hello, SilverScreen!";
    let key = test_key();
    let encrypted = encryption_service::encrypt(plaintext, &key, 1).unwrap();

    // Encrypted string must follow "v{version}:{nonce}:{ciphertext}" format.
    assert!(encrypted.starts_with("v1:"));
    let parts: Vec<&str> = encrypted.splitn(3, ':').collect();
    assert_eq!(parts.len(), 3);

    let keys = test_keys();
    let decrypted = encryption_service::decrypt(&encrypted, &keys).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encrypt_different_nonces() {
    let plaintext = "Deterministic test";
    let key = test_key();
    let enc1 = encryption_service::encrypt(plaintext, &key, 1).unwrap();
    let enc2 = encryption_service::encrypt(plaintext, &key, 1).unwrap();
    // Two encryptions of the same plaintext should produce different ciphertexts.
    assert_ne!(enc1, enc2);
}

#[test]
fn test_decrypt_wrong_key() {
    let plaintext = "Secret data";
    let key = test_key();
    let encrypted = encryption_service::encrypt(plaintext, &key, 1).unwrap();

    let mut wrong_keys = HashMap::new();
    wrong_keys.insert(1, b"abcdefghijklmnopqrstuvwxyz012345".to_vec());

    let result = encryption_service::decrypt(&encrypted, &wrong_keys);
    assert!(result.is_err());
}

#[test]
fn test_decrypt_missing_key_version() {
    let plaintext = "Test";
    let key = test_key();
    let encrypted = encryption_service::encrypt(plaintext, &key, 2).unwrap();

    // Keys map only has version 1, not version 2.
    let keys = test_keys();
    let result = encryption_service::decrypt(&encrypted, &keys);
    assert!(result.is_err());
}

#[test]
fn test_encrypt_invalid_key_length() {
    let result = encryption_service::encrypt("test", b"short", 1);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Key rotation support
// ---------------------------------------------------------------------------

#[test]
fn test_key_rotation_decrypt() {
    let key_v1 = b"0123456789abcdef0123456789abcdef".to_vec();
    let key_v2 = b"abcdefghijklmnopqrstuvwxyz012345".to_vec();

    // Encrypt with v1.
    let encrypted_v1 = encryption_service::encrypt("data_v1", &key_v1, 1).unwrap();
    // Encrypt with v2.
    let encrypted_v2 = encryption_service::encrypt("data_v2", &key_v2, 2).unwrap();

    // Keys map contains both versions.
    let mut keys = HashMap::new();
    keys.insert(1, key_v1);
    keys.insert(2, key_v2);

    assert_eq!(
        encryption_service::decrypt(&encrypted_v1, &keys).unwrap(),
        "data_v1"
    );
    assert_eq!(
        encryption_service::decrypt(&encrypted_v2, &keys).unwrap(),
        "data_v2"
    );
}

// ---------------------------------------------------------------------------
// Masking
// ---------------------------------------------------------------------------

#[test]
fn test_mask_phone() {
    let key = test_key();
    let keys = test_keys();
    let encrypted = encryption_service::encrypt("(415) 555-1234", &key, 1).unwrap();
    let masked = encryption_service::mask_phone(&encrypted, &keys);
    // Last 2 digits of the phone number should be visible.
    assert!(masked.contains("34"));
    assert!(masked.starts_with("(XXX) XXX-XX"));
}

#[test]
fn test_mask_phone_fallback() {
    let keys = test_keys();
    let masked = encryption_service::mask_phone("invalid_data", &keys);
    assert_eq!(masked, "(XXX) XXX-XXXX");
}

#[test]
fn test_mask_address() {
    let key = test_key();
    let keys = test_keys();
    let encrypted =
        encryption_service::encrypt("123 Main St, Springfield, IL 62701", &key, 1).unwrap();
    let masked = encryption_service::mask_address(&encrypted, &keys);
    assert!(masked.starts_with("***,"));
    assert!(masked.contains("Springfield"));
}

#[test]
fn test_mask_address_fallback() {
    let keys = test_keys();
    let masked = encryption_service::mask_address("invalid_data", &keys);
    assert_eq!(masked, "*****");
}

// ---------------------------------------------------------------------------
// Malformed encrypted data
// ---------------------------------------------------------------------------

#[test]
fn test_decrypt_malformed_no_colons() {
    let keys = test_keys();
    let result = encryption_service::decrypt("nocolons", &keys);
    assert!(result.is_err());
}

#[test]
fn test_decrypt_malformed_no_version_prefix() {
    let keys = test_keys();
    let result = encryption_service::decrypt("1:abc:def", &keys);
    assert!(result.is_err()); // missing 'v' prefix
}
