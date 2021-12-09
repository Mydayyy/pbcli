mod crypto;
mod privatebin;
mod api;
mod error;
mod config;

use std::ffi::OsStr;
use std::fmt::{Debug};
use url::Url;
use crate::error::{PasteError, PbResult};
use std::io::{Read, Write};
use clap::{Parser};
use crate::privatebin::{DecryptedPaste, PasteFormat};
use data_url::{DataUrl};
use serde_json::json;

const ABOUT: &str =
    "pbcli is a command line client which allows to upload and download
pastes from privatebin directly from the command line.

Project home page: https://github.com/Mydayyy/pbcli";

#[derive(Debug, Parser)]
#[clap(setting = clap::AppSettings::AllArgsOverrideSelf, version = "2.0", author = "Mydayyy <dev@mydayyy.eu>", about = ABOUT)]
struct Opts {
    #[clap(required_unless_present("host"), parse(try_from_str))]
    url: Option<Url>,

    #[clap(long, parse(try_from_str))]
    host: Option<Url>,

    #[clap(long, arg_enum, default_value = "plaintext")]
    format: PasteFormat,

    #[clap(long, default_value = "1week")]
    expire: String,

    #[clap(long)]
    json: bool,
    #[clap(long, conflicts_with = "discussion")]
    burn: bool,
    #[clap(long)]
    discussion: bool,

    #[clap(long, parse(from_os_str), value_name = "FILE")]
    download: Option<std::path::PathBuf>,
    #[clap(long)]
    overwrite: bool,
    // #[clap(long)]
    // skip_extension: bool,

    #[clap(long, parse(from_os_str), value_name = "FILE")]
    upload: Option<std::path::PathBuf>,

    #[clap(long)]
    password: Option<String>,
}

impl Opts {
    pub fn get_url(&self) -> &Url {
        self.url.as_ref().unwrap_or_else(|| self.host.as_ref().unwrap())
    }
}

fn get_stdin() -> std::io::Result<String> {
    if atty::is(atty::Stream::Stdin) {
        return Ok("".into());
    }
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer)?;
    return Ok(buffer);
}

fn create_dataurl(path: &OsStr, data: String) -> String {
    let mime = mime_guess::from_path(path).first().unwrap_or(mime_guess::mime::APPLICATION_OCTET_STREAM);
    format!("data:{};base64,{}", mime.essence_str(), data)
}

fn handle_get(opts: &Opts) -> PbResult<()> {
    let url = opts.get_url();
    let paste_id = opts.get_url().query().unwrap();
    let key = opts.get_url().fragment().ok_or(PasteError::MissingDecryptionKey)?;

    let api = api::API::new(url.clone());
    let paste = api.get_paste(paste_id)?;

    let content: DecryptedPaste;

    if let Some(pass) = &opts.password {
        content = paste.decrypt_with_password(key, pass)?;
    } else {
        match paste.decrypt(key) {
            Ok(c) => content = c,
            Err(err) => {
                if !atty::is(atty::Stream::Stdin) {
                    return Err(err);
                }

                let password = dialoguer::Password::new().with_prompt("Enter password")
                    .interact()?;
                content = paste.decrypt_with_password(key, &password)?;
            }
        }
    }

    if content.attachment.is_some() && opts.download.is_some() {
        let attachment = content.attachment.as_ref().unwrap();
        let outfile = opts.download.as_ref().unwrap();

        let url = DataUrl::process(&attachment)?;
        let (body, _) = url.decode_to_vec().unwrap();

        if outfile.exists() && !opts.overwrite {
            return Err(PasteError::FileExists);
        }

        let mut handle = std::fs::File::create(&outfile)?;

        handle.write_all(&body)?;
    }

    std::io::stdout().write_all(content.paste.as_bytes())?;

    Ok(())
}

fn handle_post(opts: &Opts) -> PbResult<()> {
    let url = opts.get_url();
    let stdin = get_stdin()?;
    let api = api::API::new(url.clone());

    let password = match &opts.password {
        None => "",
        Some(password) => password
    };

    let mut paste = DecryptedPaste {
        paste: stdin,
        attachment: None,
        attachment_name: None,
    };

    if let Some(path) = &opts.upload {
        if !path.is_file() {
            return Err(PasteError::NotAFile);
        }

        let mut handle = std::fs::File::open(path)?;
        let mut data = Vec::new();
        handle.read_to_end(&mut data)?;
        let b64_data = base64::encode(data);

        paste.attachment = Some(create_dataurl(path.as_os_str(), b64_data));
        paste.attachment_name = Some(path.file_name().ok_or(PasteError::NotAFile)?.to_string_lossy().to_string());
    }

    let res = api.post_paste(&paste, &opts.expire, password, &opts.format, opts.discussion, opts.burn)?;

    if opts.json {
        std::io::stdout().write_all(serde_json::to_string(&res)?.as_bytes())?;
    } else {
        let mut url = opts.get_url().clone();
        url.set_path("");
        url.set_query(Some(&res.id));
        url.set_fragment(Some(&res.bs58key));
        std::io::stdout().write_all(url.to_string().as_bytes())?;
        writeln!(std::io::stdout(), "");
    }

    Ok(())
}

fn main() -> PbResult<()> {
    let args = crate::config::get_args();
    let opts: Opts = Opts::parse_from(args);

    let url_has_query = opts.get_url().query().is_some();
    if url_has_query {
        handle_get(&opts)?;
    } else {
        handle_post(&opts)?;
    }

    Ok(())
}
