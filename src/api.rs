use std::str::FromStr;
use rand_chacha::rand_core::{RngCore, SeedableRng};
use reqwest::{Method, Url};
use crate::crypto::encrypt;
use crate::DecryptedPaste;
use crate::privatebin::{Paste, PasteFormat, PostPasteResponse};
use crate::error::{PasteError, PbResult};

#[derive()]
pub struct API {
    base: Url,
}


impl API {
    pub fn new(mut url: Url) -> Self {
        url.set_fragment(None);
        url.set_query(None);
        url.set_path("");
        Self { base: url }
    }

    fn make_url_query_string<I, K, V>(&self, query_string: I) -> PbResult<Url> where I: IntoIterator,
                                                                                     I::Item: std::borrow::Borrow<(K, V)>,
                                                                                     K: AsRef<str>,
                                                                                     V: AsRef<str>, {
        Ok(reqwest::Url::parse_with_params(&self.base.as_str(), query_string)?)
    }

    pub fn get_paste(&self, paste_id: &str) -> PbResult<Paste> {
        let url = self.make_url_query_string([("pasteid", paste_id)])?;
        let client = reqwest::blocking::Client::builder().build()?;
        let value: serde_json::Value = client.request(Method::from_str("GET").unwrap(), url).header("X-Requested-With", "JSONHttpRequest").send()?.json()?;
        let status: u32 = value.get("status").unwrap().as_u64().unwrap() as u32;

        match status {
            0 => Ok(value.try_into()?),
            1 => Err(PasteError::PasteNotFound),
            s => Err(PasteError::UnknownPasteStatus(s)),
        }
    }

    pub fn post_paste(&self, content: &DecryptedPaste, expire: &str, password: &str, format: &PasteFormat, discussion: bool, burn: bool) -> PbResult<PostPasteResponse> {
        let mut rng = rand_chacha::ChaCha20Rng::from_entropy();
        let mut paste_passphrase = [0u8; 32];
        let mut kdf_salt = [0u8; 8];
        let mut nonce = [0u8; 16];
        rng.fill_bytes(&mut paste_passphrase);
        rng.fill_bytes(&mut kdf_salt);
        rng.fill_bytes(&mut nonce);

        let iterations = 100000;

        let mut post_body = serde_json::json!({
            "v": 2,
            "adata": [[base64::encode(&nonce),base64::encode(&kdf_salt),100000,256,128,"aes","gcm","zlib"],format,discussion as u8,burn as u8],
            "ct": "",
            "meta": {
                "expire": expire
            }
        });
        let adata = post_body.get("adata").unwrap().to_string();
        let encrypted_content = encrypt(&serde_json::to_string(content)?, &paste_passphrase.into(), password, &kdf_salt.into(), &nonce.into(), iterations, &adata)?;
        post_body["ct"] = base64::encode(&encrypted_content).into();

        let url = self.base.clone();
        let client = reqwest::blocking::Client::builder().build()?;
        let response = client.post(url).header("X-Requested-With", "JSONHttpRequest").body::<String>(serde_json::to_string(&post_body).unwrap()).send()?;

        let mut rsv: serde_json::Value = response.json()?;
        rsv["bs58key"] = serde_json::Value::String(bs58::encode(paste_passphrase).into_string());
        let status: u32 = rsv.get("status").unwrap().as_u64().unwrap() as u32;

        match status {
            0 => Ok(serde_json::from_value::<PostPasteResponse>(rsv)?),
            1 => Err(PasteError::InvalidData),
            s => Err(PasteError::UnknownPasteStatus(s)),
        }
    }
}