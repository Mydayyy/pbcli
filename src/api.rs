use std::process::exit;
use std::str::FromStr;
use rand_chacha::rand_core::{RngCore, SeedableRng};
use reqwest::{Method, Url};
use crate::crypto::encrypt;
use crate::{DecryptedPaste};
use crate::privatebin::{Paste, PasteFormat, PostPasteResponse};
use crate::error::{PasteError, PbError, PbResult};
use crate::opts::Opts;

#[derive()]
pub struct API {
    base: Url,
    opts: Opts,
}

impl API {
    pub fn new(mut url: Url, opts: Opts) -> Self {
        url.set_fragment(None);
        url.set_query(None);
        if !url.path().ends_with("/") {
            url.set_path(&format!("{}{}", url.path(), "/"))
        }
        Self { base: url, opts }
    }

    fn get_oidc_access_token(&self) -> PbResult<String> {
        let oidc_token_endpoint = self.opts.oidc_token_url.as_ref().unwrap();
        let oidc_client_id = self.opts.oidc_client_id.as_ref().unwrap();
        let oidc_username = self.opts.oidc_username.as_ref().unwrap();
        let oidc_password = self.opts.oidc_password.as_ref().unwrap();

        let mut post_fields = std::collections::HashMap::new();
        post_fields.insert("grant_type", "password");
        post_fields.insert("client_id", oidc_client_id);
        post_fields.insert("username", oidc_username);
        post_fields.insert("password", oidc_password);

        let client = reqwest::blocking::Client::builder().build()?;
        let mut request = client.post(oidc_token_endpoint);
        request = request.form(&post_fields);

        let response = request.send()?;

        if response.status().as_u16() != 200 {
            return Err(PbError::OidcBadRequest(response.json()?));
        }

        let access_token_response: serde_json::Value = response.json()?;

        let token_type = access_token_response.get("token_type").unwrap().as_str().unwrap();
        if !token_type.eq_ignore_ascii_case("bearer") {
            return Err(PbError::InvalidTokenType(token_type.to_string()));
        }

        let token: String = access_token_response.get("access_token").unwrap().as_str().unwrap().to_string();

        Ok(token)
    }

    fn preconfigured_privatebin_request_builder(&self, method: &str, url: Url) -> PbResult<reqwest::blocking::RequestBuilder> {
        let client = reqwest::blocking::Client::builder().build()?;

        let mut request = client.request(Method::from_str(method).unwrap(), url);
        request = request.header("X-Requested-With", "JSONHttpRequest");

        if let Some(_) = &self.opts.oidc_token_url {
            let access_token = self.get_oidc_access_token()?;
            let auth_header = ["Bearer".into(), access_token].join(" ");
            request = request.header("Authorization", auth_header)
        }

        Ok(request)
    }

    pub fn get_paste(&self, paste_id: &str) -> PbResult<Paste> {
        let url = reqwest::Url::parse_with_params(&self.base.as_str(), [("pasteid", paste_id)])?;
        let value: serde_json::Value = self.preconfigured_privatebin_request_builder("GET", url)?.send()?.json()?;
        let status: u32 = value.get("status").unwrap().as_u64().unwrap() as u32;

        match status {
            0 => Ok(value.try_into()?),
            1 => Err(PasteError::PasteNotFound),
            s => Err(PasteError::UnknownPasteStatus(s)),
        }
    }

    pub fn post_paste(&self, content: &DecryptedPaste, password: &str, opts: &Opts) -> PbResult<PostPasteResponse> {
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
            "adata": [[base64::encode(&nonce),base64::encode(&kdf_salt),100000,256,128,"aes","gcm","zlib"],opts.format,opts.discussion as u8,opts.burn as u8],
            "ct": "",
            "meta": {
                "expire": opts.expire
            }
        });
        let adata = post_body.get("adata").unwrap().to_string();
        let encrypted_content = encrypt(&serde_json::to_string(content)?, &paste_passphrase.into(), password, &kdf_salt.into(), &nonce.into(), iterations, &adata)?;

        let b64_encrpyed_content = base64::encode(&encrypted_content);

        println!("paste ct size {:?}", b64_encrpyed_content.len());
        if let Some(size_limit) = &opts.size_limit {
            if b64_encrpyed_content.len() as u64 > *size_limit {
                println!("max paste size exceeded");
                let confirmation = dialoguer::Confirm::new()
                    .with_prompt("This paste exceeds your defined size limit. Continue?")
                    .interact()
                    .unwrap();

                if !confirmation {
                    exit(1)
                }
            }
        }

        post_body["ct"] = b64_encrpyed_content.into();


        let url = self.base.clone();
        let response = self.preconfigured_privatebin_request_builder("POST", url)?.body::<String>(serde_json::to_string(&post_body).unwrap()).send()?;
        let mut rsv: serde_json::Value = response.json()?;
        rsv["bs58key"] = serde_json::Value::String(bs58::encode(paste_passphrase).into_string());
        let status: u32 = rsv.get("status").unwrap().as_u64().unwrap() as u32;

        match status {
            0 => Ok(serde_json::from_value::<PostPasteResponse>(rsv)?),
            1 => Err(PasteError::InvalidData),
            s => Err(PasteError::UnknownPasteStatus(s)),
        }
    }

    pub fn base(&self) -> &Url {
        &self.base
    }
}