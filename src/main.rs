mod crypto;
mod privatebin;
mod api;
mod error;
mod config;
mod opts;

use std::io::{Read, Write};
use clap::{Parser};
use data_url::{DataUrl};
use log::log;
use parse_size::parse_size;
use crate::opts::Opts;
use crate::privatebin::{DecryptedPaste, PasteFormat};
use crate::error::{PasteError, PbResult};

fn get_stdin() -> std::io::Result<String> {
    if atty::is(atty::Stream::Stdin) {
        return Ok("".into());
    }
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer)?;
    return Ok(buffer);
}

fn create_dataurl(path: &std::ffi::OsStr, data: String) -> String {
    let mime = mime_guess::from_path(path).first().unwrap_or(mime_guess::mime::APPLICATION_OCTET_STREAM);
    format!("data:{};base64,{}", mime.essence_str(), data)
}

fn handle_get(opts: &Opts) -> PbResult<()> {
    let url = opts.get_url();
    let paste_id = opts.get_url().query().unwrap();
    let key = opts.get_url().fragment().ok_or(PasteError::MissingDecryptionKey)?;

    let api = api::API::new(url.clone(), opts.clone());
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
    let api = api::API::new(url.clone(), opts.clone());

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

    let paste_size: u64 = (paste.paste.len() + paste.attachment.as_ref().unwrap_or(&"".to_owned()).len()) as u64;
    println!("paste size {:?}", paste_size);
    if let Some(max_paste_size) = &opts.size_limit {
        if(paste_size > *max_paste_size) {
            println!("max paste size exceeded");

        }
    }

    let res = api.post_paste(&paste, &opts.expire, password, &opts.format, opts.discussion, opts.burn)?;

    if opts.json {
        std::io::stdout().write_all(serde_json::to_string(&res)?.as_bytes())?;
    } else {
        let mut url = api.base().clone();
        url.set_query(Some(&res.id));
        url.set_fragment(Some(&res.bs58key));
        std::io::stdout().write_all(url.to_string().as_bytes())?;
        writeln!(std::io::stdout(), "")?;
    }

    Ok(())
}

fn main() -> PbResult<()> {
    let args = crate::config::get_args();
    let opts: Opts = Opts::parse_from(args);

    // let size = opts.wip_arg.clone().unwrap();
    // println!("{:?}", opts.wip_arg.clone().unwrap());

    // let size = parse_size(size);
    println!("{:?}", opts.size_limit);

    let url_has_query = opts.get_url().query().is_some();
    if url_has_query {
        handle_get(&opts)?;
    } else {
        handle_post(&opts)?;
    }

    Ok(())
}
