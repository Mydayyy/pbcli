use crate::error::{PasteError, PbResult};
use crate::privatebin::Cipher;
use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::{Key, Nonce};

/// Trait implemented by any decrypt-able type (paste or comment)
pub trait Decryptable {
    /// Get ciphertext.
    /// We prefer to borrow this and not copy, because ct may be large.
    fn get_ct(&self) -> &str;
    /// Additional authenticated (but not encrypted) data.
    /// Sensitive to formatting changes.
    fn get_adata_str(&self) -> String;
    /// Cipher parameters
    fn get_cipher(&self) -> &Cipher;
}

fn derive_key(iterations: std::num::NonZeroU32, salt: &[u8], key: &[u8], out: &mut [u8]) {
    ring::pbkdf2::derive(ring::pbkdf2::PBKDF2_HMAC_SHA256, iterations, salt, key, out);
}

/// Decrypt decryptable, then attempt deserialize to requested type (DecryptedT)
pub fn decrypt_with_password<DecryptedT: serde::de::DeserializeOwned>(
    decryptable: &impl Decryptable,
    key: &[u8],
    password: &str,
) -> PbResult<DecryptedT> {
    let cipher_algo = &decryptable.get_cipher().cipher_algo;
    let cipher_mode = &decryptable.get_cipher().cipher_mode;
    let kdf_keysize = decryptable.get_cipher().kdf_keysize;

    let salt = &decryptable.get_cipher().vec_kdf_salt()?;
    let iterations = std::num::NonZeroU32::new(decryptable.get_cipher().kdf_iterations).unwrap();

    let key = [key, password.as_bytes()].concat();

    let mut derived_key = [0u8; 32];
    derive_key(iterations, salt, &key, &mut derived_key);

    match (&cipher_algo[..], &cipher_mode[..], kdf_keysize) {
        ("aes", "gcm", 256) => {
            let data = decrypt_aes_256_gcm(decryptable, &derived_key)?;
            let value: serde_json::Value = serde_json::from_slice(&data)?;
            Ok(serde_json::from_value(value)?)
        }
        _ => Err(PasteError::CipherNotImplemented {
            cipher_mode: decryptable.get_cipher().cipher_mode.clone(),
            cipher_algo: decryptable.get_cipher().cipher_algo.clone(),
            keysize: decryptable.get_cipher().kdf_keysize,
        }),
    }
}

pub fn encrypt(
    content: &str,
    key: &[u8],
    password: &str,
    salt: &[u8],
    nonce: &[u8],
    iterations: u32,
    aad: &str,
) -> PbResult<Vec<u8>> {
    let paste_blob = miniz_oxide::deflate::compress_to_vec(content.as_bytes(), 10);

    let key = [key, password.as_bytes()].concat();

    let mut derived_key = [0u8; 32];
    derive_key(
        std::num::NonZeroU32::new(iterations).unwrap(),
        salt,
        &key,
        &mut derived_key,
    );

    type Cipher = aes_gcm::AesGcm<aes_gcm::aes::Aes256, typenum::U16>;
    let cipher = Cipher::new(Key::from_slice(&derived_key));
    let payload = aes_gcm::aead::Payload {
        msg: &paste_blob,
        aad: aad.as_bytes(),
    };
    let encrypted_data = cipher.encrypt(Nonce::from_slice(nonce), payload)?;

    Ok(encrypted_data)
}

fn decrypt_aes_256_gcm(decryptable: &impl Decryptable, derived_key: &[u8]) -> PbResult<Vec<u8>> {
    type Cipher = aes_gcm::AesGcm<aes_gcm::aes::Aes256, typenum::U16>;
    let ciphertext = base64::decode(decryptable.get_ct())?;
    let nonce = decryptable.get_cipher().vec_cipher_iv()?;

    let cipher = Cipher::new(Key::from_slice(derived_key));
    let adata_str = decryptable.get_adata_str();
    let payload = aes_gcm::aead::Payload {
        msg: &ciphertext,
        aad: adata_str.as_bytes(),
    };
    let data = cipher.decrypt(Nonce::from_slice(&nonce), payload)?;
    let decompressed = miniz_oxide::inflate::decompress_to_vec(&data)?;
    Ok(decompressed)
}
