use clap::Parser;
use data_url::DataUrl;
use pbcli::api::API;
use pbcli::error::{PasteError, PbResult};
use pbcli::opts::Opts;
use pbcli::privatebin::DecryptedPaste;
use pbcli::util::check_filesize;
use serde_json::Value;
use std::io::{Read, Write};

fn get_stdin() -> std::io::Result<String> {
    if atty::is(atty::Stream::Stdin) {
        return Ok("".into());
    }
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer)
}

fn create_dataurl(path: &std::ffi::OsStr, data: String) -> String {
    let mime = mime_guess::from_path(path)
        .first()
        .unwrap_or(mime_guess::mime::APPLICATION_OCTET_STREAM);
    format!("data:{};base64,{}", mime.essence_str(), data)
}

fn handle_get(opts: &Opts) -> PbResult<()> {
    let url = opts.get_url();
    let paste_id = opts.get_url().query().unwrap();
    let key = opts
        .get_url()
        .fragment()
        .ok_or(PasteError::MissingDecryptionKey)?;

    let api = API::new(url.clone(), opts.clone());
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

                let password = dialoguer::Password::new()
                    .with_prompt("Enter password")
                    .interact()?;
                content = paste.decrypt_with_password(key, &password)?;
            }
        }
    }

    if content.attachment.is_some() && opts.download.is_some() {
        let attachment = content.attachment.as_ref().unwrap();
        let outfile = opts.download.as_ref().unwrap();

        let url = DataUrl::process(attachment)?;
        let (body, _) = url.decode_to_vec().unwrap();

        if outfile.exists() && !opts.overwrite {
            return Err(PasteError::FileExists);
        }

        let mut handle = std::fs::File::create(outfile)?;

        handle.write_all(&body)?;
    }

    std::io::stdout().write_all(content.paste.as_bytes())?;

    Ok(())
}

fn handle_post(opts: &Opts) -> PbResult<()> {
    let url = opts.get_url();
    let stdin = get_stdin()?;
    let api = API::new(url.clone(), opts.clone());

    let password = &opts.password.clone().unwrap_or_default();

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
        let metadata = handle.metadata()?;
        check_filesize(metadata.len(), opts.size_limit);

        let mut data = Vec::new();
        handle.read_to_end(&mut data)?;
        let b64_data = base64::encode(data);

        paste.attachment = Some(create_dataurl(path.as_os_str(), b64_data));
        paste.attachment_name = Some(
            path.file_name()
                .ok_or(PasteError::NotAFile)?
                .to_string_lossy()
                .to_string(),
        );
    }

    let res = api.post_paste(&paste, password, opts)?;

    if opts.json {
        let mut output: Value = serde_json::to_value(res.clone())?;
        output.as_object_mut().unwrap().insert(
            String::from("pasteurl"),
            Value::String(res.to_url(api.base()).to_string())
        );
        output.as_object_mut().unwrap().insert(
            String::from("deleteurl"),
            Value::String(res.to_delete_url(api.base()).to_string())
        );
        std::io::stdout().write_all(output.to_string().as_bytes())?;
    } else {
        std::io::stdout().write_all(res.to_url(api.base()).as_str().as_bytes())?;
        writeln!(std::io::stdout())?;
    }

    Ok(())
}

fn main() -> PbResult<()> {
    let args = pbcli::config::get_args();
    let opts: Opts = Opts::parse_from(args);

    let url_has_query = opts.get_url().query().is_some();
    if url_has_query {
        handle_get(&opts)?;
    } else {
        handle_post(&opts)?;
    }

    Ok(())
}
