use base64::DecodeError;
use data_url::DataUrlError;
use log::SetLoggerError;
use miniz_oxide::inflate::TINFLStatus;
use serde_json::Error;
use std::fmt;
use std::fmt::Formatter;

pub type PbError = PasteError;
pub type PbResult<T> = std::result::Result<T, PbError>;

#[derive(Debug)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Error))]
#[cfg_attr(feature = "uniffi", uniffi(flat_error))]
pub enum PasteError {
    CipherNotImplemented {
        cipher_algo: String,
        cipher_mode: String,
        keysize: u32,
    },
    MissingDecryptionKey,
    // BadUrl,
    PasteNotFound,
    InvalidData,
    UnknownPasteStatus(u32),
    Json(serde_json::error::Error),
    Request(reqwest::Error),
    Io(std::io::Error),
    ParseError(url::ParseError),
    Base64Error(DecodeError),
    Base58Error(bs58::decode::Error),
    Aes(aes_gcm::Error),
    Zlib(miniz_oxide::inflate::TINFLStatus),
    InvalidAttachment(data_url::DataUrlError),
    FileExists,
    NotAFile,
    InvalidTokenType(String),
    OidcBadRequest(serde_json::Value),
    LoggerInit(log::SetLoggerError),
    InvalidCertificate(String),
}

impl std::error::Error for PasteError {}

impl fmt::Display for PasteError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PasteError::CipherNotImplemented {
                cipher_algo,
                cipher_mode,
                keysize,
            } => write!(
                f,
                "Cipher not implemented algo: {} mode: {} keysize: {}",
                cipher_algo, cipher_mode, keysize
            ),
            PasteError::Json(r) => r.fmt(f),
            PasteError::Request(r) => r.fmt(f),
            PasteError::Io(r) => r.fmt(f),
            PasteError::ParseError(r) => r.fmt(f),
            PasteError::Base64Error(r) => r.fmt(f),
            PasteError::Aes(err) => err.fmt(f),
            PasteError::Zlib(err) => write!(f, "Zlib error: {:?}", err),
            PasteError::Base58Error(err) => err.fmt(f),
            PasteError::UnknownPasteStatus(err) => write!(f, "Unknown paste status: {}", err),
            PasteError::PasteNotFound => write!(f, "Invalid paste ID"),
            PasteError::MissingDecryptionKey => write!(f, "Missing decryption key"),
            // PasteError::BadUrl => write!(f, "Badly formatted url"),
            PasteError::InvalidData => write!(f, "Invalid Data"),
            PasteError::InvalidAttachment(err) => write!(f, "Invalid attachment: {:?}", err),
            PasteError::FileExists => write!(f, "File already exists. Use --overwrite to force"),
            PasteError::NotAFile => write!(f, "Given path is not a file"),
            PasteError::InvalidTokenType(token_type) => {
                write!(f, "Invalid token type: {}", token_type)
            }
            PasteError::OidcBadRequest(json) => write!(f, "{}", json),
            PasteError::LoggerInit(err) => {
                write!(f, "Failed to init logger: {}", err)
            }
            PasteError::InvalidCertificate(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<std::io::Error> for PasteError {
    fn from(err: std::io::Error) -> Self {
        PasteError::Io(err)
    }
}

impl From<serde_json::Error> for PasteError {
    fn from(err: Error) -> Self {
        PasteError::Json(err)
    }
}

impl From<url::ParseError> for PasteError {
    fn from(err: url::ParseError) -> Self {
        PasteError::ParseError(err)
    }
}

impl From<reqwest::Error> for PasteError {
    fn from(err: reqwest::Error) -> Self {
        PasteError::Request(err)
    }
}

impl From<DecodeError> for PasteError {
    fn from(err: DecodeError) -> Self {
        PasteError::Base64Error(err)
    }
}

impl From<aes_gcm::Error> for PasteError {
    fn from(err: aes_gcm::Error) -> Self {
        PasteError::Aes(err)
    }
}

impl From<miniz_oxide::inflate::TINFLStatus> for PasteError {
    fn from(err: TINFLStatus) -> Self {
        PasteError::Zlib(err)
    }
}

impl From<bs58::decode::Error> for PasteError {
    fn from(err: bs58::decode::Error) -> Self {
        PasteError::Base58Error(err)
    }
}

impl From<DataUrlError> for PasteError {
    fn from(err: DataUrlError) -> Self {
        PasteError::InvalidAttachment(err)
    }
}

impl From<SetLoggerError> for PasteError {
    fn from(err: SetLoggerError) -> Self {
        PasteError::LoggerInit(err)
    }
}
