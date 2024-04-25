use crate::error::{PasteError, PbResult};
use crate::privatebin::Cipher;
use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::{Key, Nonce};

pub trait Decryptable<'a> {
    fn get_adata_str(&self) -> String;
    fn get_cipher(&'a self) -> &'a Cipher;
    fn get_ct(&'a self) -> &'a str;
}

fn derive_key(iterations: std::num::NonZeroU32, salt: &[u8], key: &[u8], out: &mut [u8]) {
    ring::pbkdf2::derive(ring::pbkdf2::PBKDF2_HMAC_SHA256, iterations, salt, key, out);
}

pub fn decrypt_with_password<'a, DecryptedT: serde::de::DeserializeOwned>(
    pasteorcomment: &'a impl Decryptable<'a>,
    key: &[u8],
    password: &str,
) -> PbResult<DecryptedT> {
    let cipher_algo = &pasteorcomment.get_cipher().cipher_algo;
    let cipher_mode = &pasteorcomment.get_cipher().cipher_mode;
    let kdf_keysize = pasteorcomment.get_cipher().kdf_keysize;

    let salt = base64::decode(&pasteorcomment.get_cipher().kdf_salt)?;
    let iterations = std::num::NonZeroU32::new(pasteorcomment.get_cipher().kdf_iterations).unwrap();

    let key = [key, password.as_bytes()].concat();

    let mut derived_key = [0u8; 32];
    derive_key(iterations, &salt, &key, &mut derived_key);

    match (&cipher_algo[..], &cipher_mode[..], kdf_keysize) {
        ("aes", "gcm", 256) => {
            let data = decrypt_aes_256_gcm(pasteorcomment, &derived_key)?;
            let value: serde_json::Value = serde_json::from_slice(&data)?;
            Ok(serde_json::from_value(value)?)
        }
        _ => Err(PasteError::CipherNotImplemented {
            cipher_mode: pasteorcomment.get_cipher().cipher_mode.clone(),
            cipher_algo: pasteorcomment.get_cipher().cipher_algo.clone(),
            keysize: pasteorcomment.get_cipher().kdf_keysize,
        }),
    }
}

pub fn encrypt(
    content: &str,
    key: &Vec<u8>,
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

fn decrypt_aes_256_gcm<'a>(
    pasteorcomment: &'a impl Decryptable<'a>,
    derived_key: &[u8],
) -> PbResult<Vec<u8>> {
    type Cipher = aes_gcm::AesGcm<aes_gcm::aes::Aes256, typenum::U16>;
    let ciphertext = base64::decode(pasteorcomment.get_ct())?;
    let nonce = base64::decode(&pasteorcomment.get_cipher().cipher_iv)?;

    let cipher = Cipher::new(Key::from_slice(derived_key));
    let adata_str = pasteorcomment.get_adata_str();
    print!("{}", adata_str);
    let payload = aes_gcm::aead::Payload {
        msg: &ciphertext,
        aad: adata_str.as_bytes(),
    };
    let data = cipher.decrypt(Nonce::from_slice(&nonce), payload)?;
    let decompressed = miniz_oxide::inflate::decompress_to_vec(&data)?;
    Ok(decompressed)
}
