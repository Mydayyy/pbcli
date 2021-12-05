mod crypto;
mod privatebin;
mod api;
mod error;
mod config;

use std::fmt::{Debug};
use url::Url;
use crate::error::{PasteError, PbResult};
use std::io::{Read, Write};
use clap::{Parser};
use crate::privatebin::PasteFormat;

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

    #[clap(long)]
    password: Option<String>,
}

impl Opts {
    pub fn get_url(&self) -> &Url {
        self.url.as_ref().unwrap_or_else(|| self.host.as_ref().unwrap())
    }
}

fn get_stdin() -> std::io::Result<String> {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer)?;
    return Ok(buffer);
}

fn handle_get(opts: &Opts) -> PbResult<()> {
    let url = opts.get_url();
    let paste_id = opts.get_url().query().unwrap();
    let key = opts.get_url().fragment().ok_or(PasteError::MissingDecryptionKey)?;

    let api = api::API::new(url.clone());
    let paste = api.get_paste(paste_id)?;

    if let Some(pass) = &opts.password {
        let content = paste.decrypt_with_password(key, pass);
        std::io::stdout().write_all(content?.paste.as_bytes())?;
        return Ok(());
    }

    let content = paste.decrypt(key);

    if let Err(err) = content {
        if !atty::is(atty::Stream::Stdin) {
            return Err(err);
        }

        let password = dialoguer::Password::new().with_prompt("Enter password")
            .interact()?;
        let content = paste.decrypt_with_password(key, &password)?;
        std::io::stdout().write_all(content.paste.as_bytes())?;
        return Ok(());
    }

    std::io::stdout().write_all(content?.paste.as_bytes())?;

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
    let res = api.post_paste(&stdin, &opts.expire, password, &opts.format, opts.discussion, opts.burn)?;

    if opts.json {
        std::io::stdout().write_all(serde_json::to_string(&res)?.as_bytes())?;
    } else {
        let mut url = opts.get_url().clone();
        url.set_path("");
        url.set_query(Some(&res.id));
        url.set_fragment(Some(&res.bs58key));
        std::io::stdout().write_all(url.to_string().as_bytes())?;
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
