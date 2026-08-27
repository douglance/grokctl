//! Electron safe-storage decryption primitives.

use aes::cipher::{BlockModeDecrypt, KeyIvInit, block_padding::Pkcs7};
use pbkdf2::pbkdf2_hmac;
use sha1::Sha1;
use zeroize::Zeroizing;

const PREFIX: &[u8] = b"v10";
const SALT: &[u8] = b"saltysalt";
const ROUNDS: u32 = 1_003;
const IV: [u8; 16] = [b' '; 16];

pub(super) fn decrypt_v10(ciphertext: &[u8], password: &[u8]) -> Option<String> {
    let mut key = Zeroizing::new([0_u8; 16]);
    pbkdf2_hmac::<Sha1>(password, SALT, ROUNDS, key.as_mut());
    decrypt_v10_with_key(ciphertext, key.as_ref())
}

fn decrypt_v10_with_key(ciphertext: &[u8], key: &[u8]) -> Option<String> {
    let encrypted = ciphertext.strip_prefix(PREFIX)?;
    let key = <&[u8; 16]>::try_from(key).ok()?;
    if encrypted.is_empty() || encrypted.len() % 16 != 0 {
        return None;
    }
    let mut buffer = Zeroizing::new(encrypted.to_vec());
    let plaintext = cbc::Decryptor::<aes::Aes128>::new(key.into(), (&IV).into())
        .decrypt_padded::<Pkcs7>(buffer.as_mut())
        .ok()?;
    String::from_utf8(plaintext.to_vec()).ok()
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    use super::*;

    #[test]
    fn decrypts_chromium_macos_v10_ciphertext() {
        let ciphertext = STANDARD
            .decode("djEwwqTpnXPs2YcGDFMu2jc4J/m1BZfznPWMWgGxm2+x/PGpLxWCKti1bc3mY5oZq9/5")
            .ok();
        let plaintext = ciphertext.as_deref().and_then(|value| decrypt_v10(value, b"peanuts"));

        assert_eq!(plaintext.as_deref(), Some(r#"{"baseUrl":"http://127.0.0.1:1340"}"#));
    }
}
