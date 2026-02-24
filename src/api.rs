use crate::crypto::{encrypt, Decryptable};
use crate::error::{PasteError, PbError, PbResult};
use crate::opts::Opts;
use crate::privatebin::{Comment, DecryptedComment, Paste, PostCommentResponse, PostPasteResponse};
use crate::util::check_filesize;
use crate::DecryptedPaste;
use rand_chacha::rand_core::{RngCore, SeedableRng};
use reqwest::tls::Certificate;
use reqwest::{Method, Url};
use scraper::{Html, Selector};
use std::str::FromStr;
use std::time::Duration;

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct API {
    base: Url,
    opts: Opts,
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
impl API {
    #[cfg_attr(feature = "uniffi", uniffi::constructor)]
    pub fn new(mut url: Url, opts: Opts) -> Self {
        url.set_fragment(None);
        url.set_query(None);
        if !url.path().ends_with('/') {
            url.set_path(&format!("{}{}", url.path(), "/"))
        }
        Self { base: url, opts }
    }
}

impl API {
    fn build_client(&self) -> PbResult<reqwest::blocking::Client> {
        let mut builder = reqwest::blocking::Client::builder();

        let timeout_secs = self.opts.timeout.unwrap_or(30);
        builder = builder
            .connect_timeout(Duration::from_secs(timeout_secs))
            .timeout(Duration::from_secs(timeout_secs * 4));

        if self.opts.insecure {
            builder = builder.danger_accept_invalid_certs(true);
        }

        if let Some(ref ca_path) = self.opts.ca_cert {
            let pem = std::fs::read(ca_path).map_err(|e| {
                PbError::InvalidCertificate(format!(
                    "failed to read CA cert {}: {}",
                    ca_path.display(),
                    e
                ))
            })?;
            for cert in pem_certs_from_bundle(&pem)? {
                builder = builder.add_root_certificate(cert);
            }
        }

        Ok(builder.build()?)
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

        let client = self.build_client()?;
        let mut request = client.post(oidc_token_endpoint);
        request = request.form(&post_fields);

        let response = request.send()?;

        if response.status().as_u16() != 200 {
            return Err(PbError::OidcBadRequest(response.json()?));
        }

        let access_token_response: serde_json::Value = response.json()?;

        let token_type = access_token_response
            .get("token_type")
            .unwrap()
            .as_str()
            .unwrap();
        if !token_type.eq_ignore_ascii_case("bearer") {
            return Err(PbError::InvalidTokenType(token_type.to_string()));
        }

        let token: String = access_token_response
            .get("access_token")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();

        Ok(token)
    }

    fn preconfigured_privatebin_request_builder(
        &self,
        method: &str,
        url: Url,
        json_request: bool,
    ) -> PbResult<reqwest::blocking::RequestBuilder> {
        let client = self.build_client()?;

        let mut request = client.request(Method::from_str(method).unwrap(), url);
        if json_request {
            request = request.header("X-Requested-With", "JSONHttpRequest");
        }

        if self.opts.oidc_token_url.is_some() {
            let access_token = self.get_oidc_access_token()?;
            let auth_header = ["Bearer".into(), access_token].join(" ");
            request = request.header("Authorization", auth_header)
        }

        Ok(request)
    }
}

fn pem_certs_from_bundle(pem: &[u8]) -> PbResult<Vec<Certificate>> {
    let pem_str = std::str::from_utf8(pem)
        .map_err(|e| PbError::InvalidCertificate(format!("CA cert is not valid UTF-8: {}", e)))?;
    let mut certs = Vec::new();
    let mut current = String::new();
    let mut in_cert = false;
    for line in pem_str.lines() {
        if line.contains("BEGIN CERTIFICATE") {
            in_cert = true;
            current.clear();
            current.push_str(line);
            current.push('\n');
        } else if line.contains("END CERTIFICATE") {
            current.push_str(line);
            current.push('\n');
            certs.push(
                Certificate::from_pem(current.as_bytes()).map_err(|e| {
                    PbError::InvalidCertificate(format!("invalid certificate in bundle: {}", e))
                })?,
            );
            in_cert = false;
        } else if in_cert {
            current.push_str(line);
            current.push('\n');
        }
    }
    Ok(certs)
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
impl API {
    pub fn get_paste(&self, paste_id: &str) -> PbResult<Paste> {
        let url = reqwest::Url::parse_with_params(self.base.as_str(), [("pasteid", paste_id)])?;
        let value: serde_json::Value = self
            .preconfigured_privatebin_request_builder("GET", url, true)?
            .send()?
            .json()?;
        let status: u32 = value.get("status").unwrap().as_u64().unwrap() as u32;

        match status {
            0 => Ok(serde_json::from_value(value)?),
            1 => Err(PasteError::PasteNotFound),
            s => Err(PasteError::UnknownPasteStatus(s)),
        }
    }

    pub fn post_paste(
        &self,
        content: &DecryptedPaste,
        password: &str,
        opts: &Opts,
    ) -> PbResult<PostPasteResponse> {
        let mut rng = rand_chacha::ChaCha20Rng::from_entropy();
        let mut paste_passphrase = [0u8; 32];
        rng.fill_bytes(&mut paste_passphrase);

        let mut paste = Paste {
            v: 2,
            ..Default::default()
        };
        paste.adata.format = opts.format;
        paste.adata.discuss = opts.discussion as u8;
        paste.adata.burn = opts.burn as u8;
        paste.meta.expire = Some(opts.expire.clone());

        let cipher = &paste.adata.cipher;

        let encrypted_content = encrypt(
            &serde_json::to_string(content)?,
            &paste_passphrase,
            password,
            &cipher.vec_kdf_salt()?,
            &cipher.vec_cipher_iv()?,
            cipher.kdf_iterations,
            &paste.get_adata_str(),
        )?;

        let b64_encrpyed_content = base64::encode(encrypted_content);
        check_filesize(b64_encrpyed_content.len() as u64, opts.size_limit);
        paste.ct = b64_encrpyed_content;

        let url = self.base.clone();
        let response = self
            .preconfigured_privatebin_request_builder("POST", url, true)?
            .body::<String>(serde_json::to_string(&paste).unwrap())
            .send()?;
        let mut rsv: serde_json::Value = response.json()?;
        rsv["bs58key"] = serde_json::Value::String(bs58::encode(paste_passphrase).into_string());
        rsv["baseurl"] = serde_json::Value::String(self.base.to_string());
        let status: u32 = rsv.get("status").unwrap().as_u64().unwrap() as u32;

        match status {
            0 => Ok(serde_json::from_value::<PostPasteResponse>(rsv)?),
            1 => Err(PasteError::InvalidData),
            s => Err(PasteError::UnknownPasteStatus(s)),
        }
    }

    pub fn post_comment(
        &self,
        content: &DecryptedComment,
        paste_id: &str,
        parent_id: &str,
        bs58key: &str,
        password: &str,
        opts: &Opts,
    ) -> PbResult<PostCommentResponse> {
        let mut comment = Comment {
            v: 2,
            pasteid: paste_id.into(),
            parentid: parent_id.into(),
            ..Default::default()
        };
        let cipher = &comment.adata;
        let paste_passphrase = bs58::decode(bs58key).into_vec()?;

        let encrypted_content = encrypt(
            &serde_json::to_string(content)?,
            &paste_passphrase,
            password,
            &cipher.vec_kdf_salt()?,
            &cipher.vec_cipher_iv()?,
            cipher.kdf_iterations,
            &comment.get_adata_str(),
        )?;

        let b64_encrpyed_content = base64::encode(encrypted_content);
        check_filesize(b64_encrpyed_content.len() as u64, opts.size_limit);
        comment.ct = b64_encrpyed_content;

        let url = self.base.clone();
        let response = self
            .preconfigured_privatebin_request_builder("POST", url, true)?
            .body::<String>(serde_json::to_string(&comment).unwrap())
            .send()?;
        let rsv: serde_json::Value = response.json()?;
        let status: u32 = rsv.get("status").unwrap().as_u64().unwrap() as u32;

        match status {
            0 => Ok(serde_json::from_value::<PostCommentResponse>(rsv)?),
            1 => Err(PasteError::InvalidData),
            s => Err(PasteError::UnknownPasteStatus(s)),
        }
    }

    pub fn scrape_expiries(&self) -> PbResult<Vec<String>> {
        let url = self.base.clone();
        let response = self
            .preconfigured_privatebin_request_builder("GET", url, false)?
            .send()?;
        response.error_for_status_ref()?;
        let html = response.text()?;
        let document = Html::parse_document(&html);
        let expiries_selector = Selector::parse("#expiration + ul > li > a").unwrap();
        let mut expiries = Vec::new();
        for expiry_anchor in document.select(&expiries_selector) {
            if let Some(expiry) = expiry_anchor.attr("data-expiration") {
                expiries.push(expiry.to_string());
            }
        }
        Ok(expiries)
    }

    pub fn base(&self) -> Url {
        self.base.clone()
    }
}
