use crate::error::PbResult;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_with::skip_serializing_none;
use std::io::ErrorKind;
use url::Url;

#[derive(Deserialize, Debug, Serialize)]
pub enum CompressionType {
    #[serde(rename = "none")]
    None,

    #[serde(rename = "zlib")]
    Zlib,
}

#[derive(clap::ArgEnum, Deserialize, Debug, Serialize, Clone)]
pub enum PasteFormat {
    #[serde(rename = "plaintext")]
    Plaintext,

    #[serde(rename = "syntaxhighlighting")]
    Syntax,

    #[serde(rename = "markdown")]
    Markdown,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Paste {
    pub status: i32,
    pub id: String,
    pub url: String,
    pub v: i32,
    pub ct: String,
    pub meta: Meta,
    pub adata: Data,

    #[serde(skip)]
    pub adata_str: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Meta {
    pub created: i32,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Data {
    pub cipher: Cipher,
    pub format: PasteFormat,
    pub discuss: i8,
    pub burn: i8,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Cipher {
    pub cipher_iv: String,
    pub kdf_salt: String,
    pub kdf_iterations: u32,
    pub kdf_keysize: u32,
    pub cipher_tag_size: u32,
    pub cipher_algo: String,
    pub cipher_mode: String,
    pub compression_type: CompressionType,
    // test: String,
}

#[skip_serializing_none]
#[derive(Deserialize, Debug, Serialize)]
pub struct DecryptedPaste {
    pub paste: String,
    pub attachment: Option<String>,
    pub attachment_name: Option<String>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct PostPasteResponse {
    pub deletetoken: String,
    pub id: String,
    pub status: u32,
    pub url: String,
    pub baseurl: Url,
    pub bs58key: String,
}

impl PostPasteResponse {
    /// Return full paste url, i.e (base + ?id + #bs58key)
    pub fn to_paste_url(&self) -> url::Url {
        let mut paste_url: url::Url = self.baseurl.clone();
        paste_url.set_query(Some(&self.id));
        paste_url.set_fragment(Some(&self.bs58key));
        paste_url
    }
    /// Return url that can be used to delete paste
    pub fn to_delete_url(&self) -> url::Url {
        let mut delete_url: url::Url = self.baseurl.clone();
        delete_url
            .query_pairs_mut()
            .append_pair("pasteid", &self.id)
            .append_pair("deletetoken", &self.deletetoken);
        delete_url
    }
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
    pub fn is_success(&self) -> bool {
        self.status == 0
    }
}

impl Paste {
    pub fn decrypt(&self, bs58_key: &str) -> PbResult<DecryptedPaste> {
        self.decrypt_with_password(bs58_key, "")
    }

    pub fn decrypt_with_password(
        &self,
        bs58_key: &str,
        password: &str,
    ) -> PbResult<DecryptedPaste> {
        let key = bs58::decode(bs58_key).into_vec()?;
        crate::crypto::decrypt_with_password(self, &key, password)
    }
}

impl TryFrom<serde_json::Value> for Paste {
    type Error = crate::error::PbError;

    fn try_from(value: Value) -> PbResult<Self> {
        let adata = value.get("adata").ok_or(std::io::Error::new(
            ErrorKind::InvalidData,
            "Cannot get adata in try_from",
        ))?;
        let adata_str = serde_json::to_string(adata)?;

        let mut paste = serde_json::from_value::<Paste>(value)?;

        paste.adata_str = adata_str;

        Ok(paste)
    }
}
