use std::collections::HashMap;

use crate::crypto::Decryptable;
use crate::error::PbResult;
use serde::ser::{SerializeTuple, Serializer};
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use serde_with::skip_serializing_none;
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
    pub comments: Option<Vec<Comment>>,
}

impl Decryptable for Paste {
    fn get_ct(&self) -> &str {
        &self.ct
    }
    fn get_cipher(&self) -> &Cipher {
        &self.adata.cipher
    }
    fn get_adata_str(&self) -> String {
        serde_json::to_string(&self.adata).unwrap()
    }
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Comment {
    pub status: Option<i32>,
    pub id: String,
    pub pasteid: String,
    pub parentid: String,
    pub url: Option<String>,
    pub v: i32,
    pub ct: String,
    pub meta: Meta,
    pub adata: Cipher,
}

impl Decryptable for Comment {
    fn get_ct(&self) -> &str {
        &self.ct
    }
    fn get_cipher(&self) -> &Cipher {
        &self.adata
    }
    fn get_adata_str(&self) -> String {
        serde_json::to_string(&self.adata).unwrap()
    }
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Meta {
    pub created: i32,
}

#[derive(Deserialize, Debug)]
pub struct Data {
    pub cipher: Cipher,
    pub format: PasteFormat,
    pub discuss: i8,
    pub burn: i8,
}

#[derive(Deserialize, Debug)]
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

#[skip_serializing_none]
#[derive(Default, Deserialize, Debug, Serialize)]
pub struct DecryptedComment {
    pub comment: String,
    pub nickname: Option<String>,
}

/// comment.id -> decrypted_comment
type DecryptedCommentsMap = HashMap<String, DecryptedComment>;

/// comment.id -> [children comment.id]
type CommentsAdjacencyMap = HashMap<String, Vec<String>>;

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

    /// Returns a mapping: comment.id -> decrypted_comment
    pub fn decrypt_comments(&self, bs58_key: &str) -> PbResult<DecryptedCommentsMap> {
        self.decrypt_comments_with_password(bs58_key, "")
    }

    /// Returns a mapping: comment.id -> decrypted_comment
    pub fn decrypt_comments_with_password(
        &self,
        bs58_key: &str,
        password: &str,
    ) -> PbResult<DecryptedCommentsMap> {
        let mut decrypted_comments = HashMap::new();
        if let Some(comments) = &self.comments {
            for comment in comments {
                let id = comment.id.clone();
                decrypted_comments.insert(id, comment.decrypt_with_password(bs58_key, password)?);
            }
        };
        Ok(decrypted_comments)
    }

    /// Returns a mapping: comment.id -> [children comment.id]
    pub fn comments_adjacency_map(&self) -> PbResult<CommentsAdjacencyMap> {
        let mut comment_adjacency: CommentsAdjacencyMap = HashMap::new();
        if let Some(comments) = &self.comments {
            for c in comments {
                let id = c.id.clone();
                let parentid = c.parentid.clone();
                comment_adjacency.entry(parentid).or_default().push(id);
            }
        }
        Ok(comment_adjacency)
    }

    /// Returns a formatted json tree of decrypted comments
    pub fn comments_formatted_tree(
        &self,
        decrypted_comments: &DecryptedCommentsMap,
        comment_adjacency: &CommentsAdjacencyMap,
    ) -> PbResult<String> {
        fn format_comments_below_id(
            id: &str,
            decrypted_comments: &DecryptedCommentsMap,
            comment_adjacency: &CommentsAdjacencyMap,
        ) -> serde_json::Value {
            let formatted_children: Vec<serde_json::Value> = comment_adjacency
                .get(id)
                .unwrap_or(&Vec::new())
                .iter()
                .map(|child_id| {
                    format_comments_below_id(child_id, decrypted_comments, comment_adjacency)
                })
                .collect();
            json!({
                id: {
                    "comment": decrypted_comments.get(id).unwrap_or(&DecryptedComment::default()).comment,
                    "nickname": decrypted_comments.get(id).unwrap_or(&DecryptedComment::default()).nickname,
                    "replies": formatted_children
                }
            })
        }
        let top_level = format_comments_below_id(&self.id, decrypted_comments, comment_adjacency);
        serde_json::to_string_pretty(&top_level).map_err(|e| e.into())
    }
}

impl Comment {
    pub fn decrypt(&self, bs58_key: &str) -> PbResult<DecryptedComment> {
        self.decrypt_with_password(bs58_key, "")
    }

    pub fn decrypt_with_password(
        &self,
        bs58_key: &str,
        password: &str,
    ) -> PbResult<DecryptedComment> {
        let key = bs58::decode(bs58_key).into_vec()?;
        crate::crypto::decrypt_with_password(self, &key, password)
    }
}

/// Data struct needs to be serialized as an ordered array (not object),
/// so we implement custom serialization.
impl Serialize for Data {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_tuple(4)?;
        s.serialize_element(&self.cipher)?;
        s.serialize_element(&self.format)?;
        s.serialize_element(&self.discuss)?;
        s.serialize_element(&self.burn)?;
        s.end()
    }
}

/// Cipher struct needs to be serialized as an ordered array (not object),
/// so we implement custom serialization.
impl Serialize for Cipher {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_tuple(8)?;
        s.serialize_element(&self.cipher_iv)?;
        s.serialize_element(&self.kdf_salt)?;
        s.serialize_element(&self.kdf_iterations)?;
        s.serialize_element(&self.kdf_keysize)?;
        s.serialize_element(&self.cipher_tag_size)?;
        s.serialize_element(&self.cipher_algo)?;
        s.serialize_element(&self.cipher_mode)?;
        s.serialize_element(&self.compression_type)?;
        s.end()
    }
}
