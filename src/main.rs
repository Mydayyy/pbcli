use clap::Parser;
use data_url::DataUrl;
use pbcli::api::API;
use pbcli::error::{PasteError, PbResult};
use pbcli::opts::Opts;
use pbcli::privatebin::{DecryptedComment, DecryptedCommentsMap, DecryptedPaste};
use pbcli::util::check_filesize;
use serde_json::Value;
use std::io::IsTerminal;
use std::io::{Read, Write};
use log::LevelFilter;

mod logger;

fn get_stdin() -> std::io::Result<String> {
    if std::io::stdin().is_terminal() {
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
    let fragment = opts
        .get_url()
        .fragment()
        .ok_or(PasteError::MissingDecryptionKey)?;
    // '-' character may be found at start of fragment. This should be stripped.
    // It is used to activate "warn before read" feature for burn on read pastes.
    let key = fragment.strip_prefix('-').unwrap_or(fragment);

    let api = API::new(url.clone(), opts.clone());
    let paste = api.get_paste(paste_id)?;

    let content: DecryptedPaste;
    let comments: DecryptedCommentsMap;

    if let Some(pass) = &opts.password {
        content = paste.decrypt_with_password(key, pass)?;
        comments = paste.decrypt_comments_with_password(key, pass)?;
    } else {
        match paste.decrypt(key) {
            Ok(c) => {
                content = c;
                comments = paste.decrypt_comments(key)?;
            }
            Err(err) => {
                if !std::io::stdin().is_terminal() {
                    return Err(err);
                }

                let password = dialoguer::Password::new()
                    .with_prompt("Enter password")
                    .interact()?;
                content = paste.decrypt_with_password(key, &password)?;
                comments = paste.decrypt_comments_with_password(key, &password)?;
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

    if !opts.json {
        std::io::stdout().write_all(content.paste.as_bytes())?;
    } else {
        let mut output: Value = serde_json::to_value(content)?;
        if !comments.is_empty() {
            let comments_trees =
                paste.comments_formatted_json_trees(&comments, &paste.comments_adjacency_map()?)?;
            output["comments"] = serde_json::from_str(&comments_trees)?;
        }
        std::io::stdout().write_all(serde_json::to_string_pretty(&output)?.as_bytes())?;
    }

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
        output["pasteurl"] = Value::String(res.to_paste_url().to_string());
        output["deleteurl"] = Value::String(res.to_delete_url().to_string());
        std::io::stdout().write_all(serde_json::to_string_pretty(&output)?.as_bytes())?;
    } else {
        std::io::stdout().write_all(res.to_paste_url().as_str().as_bytes())?;
        writeln!(std::io::stdout())?;
    }

    Ok(())
}

fn handle_comment(opts: &Opts) -> PbResult<()> {
    let url = opts.get_url();
    let paste_id = url.query().unwrap();
    let fragment = url.fragment().ok_or(PasteError::MissingDecryptionKey)?;
    // '-' character may be found at start of fragment. This should be stripped.
    // It is used to activate "warn before read" feature for burn on read pastes.
    let key = fragment.strip_prefix('-').unwrap_or(fragment);

    let stdin = get_stdin()?;
    let api = API::new(url.clone(), opts.clone());
    let content = DecryptedComment {
        comment: stdin,
        nickname: opts.comment_as.clone(),
    };
    let parent_id = &opts.comment_to.clone().unwrap_or(paste_id.into());
    let password = &opts.password.clone().unwrap_or_default();

    api.post_comment(&content, paste_id, parent_id, key, password, opts)?;

    Ok(())
}

fn main() -> PbResult<()> {
    crate::logger::SimpleLogger::init()?;
    if std::env::args_os().find(|arg| arg == "--debug").is_some() {
        log::set_max_level(log::LevelFilter::Debug);
    }

    let args = pbcli::config::get_args();
    let opts: Opts = Opts::parse_from(args);

    let url_has_query = opts.get_url().query().is_some();
    if url_has_query {
        if opts.comment {
            handle_comment(&opts)?;
            // show paste with comments after commenting
            if opts.json {
                handle_get(&opts)?;
            }
            return Ok(());
        }
        handle_get(&opts)?;
    } else {
        handle_post(&opts)?;
    }

    Ok(())
}
