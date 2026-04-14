use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use std::collections::HashMap;

use crate::errors::AppError;

// ---------------------------------------------------------------------------
// Encrypt
// ---------------------------------------------------------------------------

/// Encrypts plaintext using AES-256-GCM and returns a versioned, portable string:
/// `"v{version}:{nonce_base64}:{ciphertext_base64}"`.
///
/// The key must be exactly 32 bytes (256 bits).
pub fn encrypt(plaintext: &str, key: &[u8], version: u32) -> Result<String, AppError> {
    if key.len() != 32 {
        return Err(AppError::InternalError(
            "Encryption key must be exactly 32 bytes".to_string(),
        ));
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| AppError::InternalError(format!("Failed to create cipher: {}", e)))?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| AppError::InternalError(format!("Encryption failed: {}", e)))?;

    let nonce_b64 = BASE64.encode(nonce_bytes);
    let ct_b64 = BASE64.encode(&ciphertext);

    Ok(format!("v{}:{}:{}", version, nonce_b64, ct_b64))
}

// ---------------------------------------------------------------------------
// Decrypt
// ---------------------------------------------------------------------------

/// Decrypts a versioned encrypted string produced by [`encrypt`].
///
/// Looks up the correct key version from `keys` to support key rotation.
pub fn decrypt(encrypted: &str, keys: &HashMap<u32, Vec<u8>>) -> Result<String, AppError> {
    let parts: Vec<&str> = encrypted.splitn(3, ':').collect();
    if parts.len() != 3 {
        return Err(AppError::BadRequest(
            "Invalid encrypted data format".to_string(),
        ));
    }

    let version: u32 = parts[0]
        .strip_prefix('v')
        .ok_or_else(|| AppError::BadRequest("Missing version prefix".to_string()))?
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid version number".to_string()))?;

    let key = keys
        .get(&version)
        .ok_or_else(|| {
            AppError::InternalError(format!("No key found for version {}", version))
        })?;

    if key.len() != 32 {
        return Err(AppError::InternalError(
            "Decryption key must be exactly 32 bytes".to_string(),
        ));
    }

    let nonce_bytes = BASE64
        .decode(parts[1])
        .map_err(|e| AppError::BadRequest(format!("Invalid nonce encoding: {}", e)))?;
    let ciphertext = BASE64
        .decode(parts[2])
        .map_err(|e| AppError::BadRequest(format!("Invalid ciphertext encoding: {}", e)))?;

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| AppError::InternalError(format!("Failed to create cipher: {}", e)))?;

    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| {
            AppError::InternalError(
                "Decryption failed -- wrong key or corrupted data".to_string(),
            )
        })?;

    String::from_utf8(plaintext)
        .map_err(|e| AppError::InternalError(format!("Decrypted data is not valid UTF-8: {}", e)))
}

// ---------------------------------------------------------------------------
// Masking helpers
// ---------------------------------------------------------------------------

/// Decrypts an encrypted phone number and masks it as `"(XXX) XXX-XX{last2}"`.
///
/// If decryption fails the function returns `"(XXX) XXX-XXXX"` rather than
/// propagating an error, so callers always get a safe display value.
pub fn mask_phone(encrypted_phone: &str, keys: &HashMap<u32, Vec<u8>>) -> String {
    match decrypt(encrypted_phone, keys) {
        Ok(phone) => {
            let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() >= 2 {
                let last2 = &digits[digits.len() - 2..];
                format!("(XXX) XXX-XX{}", last2)
            } else {
                "(XXX) XXX-XXXX".to_string()
            }
        }
        Err(_) => "(XXX) XXX-XXXX".to_string(),
    }
}

/// Decrypts an encrypted address and masks it, showing only city/state.
///
/// Expects a comma-separated address where the last segments are
/// "City, State ZIP". Falls back to full mask on any error.
pub fn mask_address(encrypted_address: &str, keys: &HashMap<u32, Vec<u8>>) -> String {
    match decrypt(encrypted_address, keys) {
        Ok(address) => {
            let parts: Vec<&str> = address.split(',').map(|s| s.trim()).collect();
            if parts.len() >= 2 {
                // Show the last two parts (typically city, state/zip)
                let city_state = parts[parts.len() - 2..].join(", ");
                format!("***, {}", city_state)
            } else {
                "*****".to_string()
            }
        }
        Err(_) => "*****".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        *b"0123456789abcdef0123456789abcdef"
    }

    fn key_map() -> HashMap<u32, Vec<u8>> {
        let mut m = HashMap::new();
        m.insert(1, test_key().to_vec());
        m
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = test_key();
        let plaintext = "Hello, SilverScreen!";
        let encrypted = encrypt(plaintext, &key, 1).unwrap();
        assert!(encrypted.starts_with("v1:"));
        let decrypted = decrypt(&encrypted, &key_map()).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_different_nonce_each_time() {
        let key = test_key();
        let e1 = encrypt("same input", &key, 1).unwrap();
        let e2 = encrypt("same input", &key, 1).unwrap();
        assert_ne!(e1, e2, "Same plaintext must produce different ciphertext");
    }

    #[test]
    fn test_encrypt_wrong_key_length() {
        let short_key = b"too_short";
        let result = encrypt("test", short_key, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_invalid_format() {
        let result = decrypt("not_valid_format", &key_map());
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_missing_version_prefix() {
        let result = decrypt("1:abc:def", &key_map());
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_unknown_key_version() {
        let key = test_key();
        let encrypted = encrypt("test", &key, 1).unwrap();
        let wrong_map: HashMap<u32, Vec<u8>> = HashMap::new();
        let result = decrypt(&encrypted, &wrong_map);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let key = test_key();
        let encrypted = encrypt("secret", &key, 1).unwrap();
        let mut wrong_map = HashMap::new();
        wrong_map.insert(1, vec![0u8; 32]);
        let result = decrypt(&encrypted, &wrong_map);
        assert!(result.is_err());
    }

    #[test]
    fn test_mask_phone_shows_last_two_digits() {
        let key = test_key();
        let encrypted = encrypt("555-123-4567", &key, 1).unwrap();
        let masked = mask_phone(&encrypted, &key_map());
        assert_eq!(masked, "(XXX) XXX-XX67");
    }

    #[test]
    fn test_mask_phone_short_number() {
        let key = test_key();
        let encrypted = encrypt("5", &key, 1).unwrap();
        let masked = mask_phone(&encrypted, &key_map());
        assert_eq!(masked, "(XXX) XXX-XXXX");
    }

    #[test]
    fn test_mask_phone_invalid_encrypted_data() {
        let masked = mask_phone("garbage", &key_map());
        assert_eq!(masked, "(XXX) XXX-XXXX");
    }

    #[test]
    fn test_mask_address_shows_city_state() {
        let key = test_key();
        let encrypted = encrypt("123 Main St, Springfield, IL 62701", &key, 1).unwrap();
        let masked = mask_address(&encrypted, &key_map());
        assert!(masked.starts_with("***, "));
        assert!(masked.contains("Springfield"));
        assert!(masked.contains("IL 62701"));
    }

    #[test]
    fn test_mask_address_single_part() {
        let key = test_key();
        let encrypted = encrypt("nowhere", &key, 1).unwrap();
        let masked = mask_address(&encrypted, &key_map());
        assert_eq!(masked, "*****");
    }

    #[test]
    fn test_mask_address_invalid_encrypted_data() {
        let masked = mask_address("garbage", &key_map());
        assert_eq!(masked, "*****");
    }

    #[test]
    fn test_encrypt_empty_string() {
        let key = test_key();
        let encrypted = encrypt("", &key, 1).unwrap();
        let decrypted = decrypt(&encrypted, &key_map()).unwrap();
        assert_eq!(decrypted, "");
    }

    #[test]
    fn test_encrypt_unicode() {
        let key = test_key();
        let plaintext = "日本語テスト 🎬";
        let encrypted = encrypt(plaintext, &key, 1).unwrap();
        let decrypted = decrypt(&encrypted, &key_map()).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
