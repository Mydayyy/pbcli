use clap::Parser;
use data_url::DataUrl;
use url::Url;
use reqwest::blocking::Client;
use pbcli::api::API;
use pbcli::error::{PasteError, PbResult};
use pbcli::opts::Opts;
use pbcli::privatebin::{DecryptedComment, DecryptedCommentsMap, DecryptedPaste};
use pbcli::util::check_filesize;
use serde_json::Value;
use std::time::Duration;
use std::ffi::OsString;
use std::io::IsTerminal;
use std::io::{Read, Write};

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

    if let (Some(attachment), Some(outfile)) = (content.attachment.as_ref(), opts.download.as_ref())
    {
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

fn shorten_via_privatebin(opts: &Opts, long_url: &str) -> PbResult<String> {
    fn try_method(opts: &Opts, long_url: &str, method: &str) -> PbResult<String> {
        let encoded = url::form_urlencoded::byte_serialize(long_url.as_bytes()).collect::<String>();

        // Always shorten on the same host as --host
        let mut endpoint = opts.get_url().clone();
        endpoint.set_fragment(None);
        endpoint.set_path("/");
        endpoint.set_query(Some(&format!("{method}&link={encoded}")));

        let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
        let resp_text = client
        .get(endpoint.clone())
        .send()?
        .error_for_status()?
        .text()?;

        let text = resp_text.trim();

        log::debug!("shortener ({}) GET: {}", method, endpoint);
        log::debug!("shortener ({}) raw response: {}", method, text);

        // JSON first (some proxies return JSON)
        if let Ok(v) = serde_json::from_str::<Value>(text) {
            if let Some(s) = v.get("shorturl").and_then(|x| x.as_str()) {
                return Ok(s.to_string());
            }
            if let Some(s) = v.get("url").and_then(|x| x.as_str()) {
                return Ok(s.to_string());
            }
        }

        // HTML: accept only <a id="pasteurl" href="...">
        if let Some(id_pos) = text.find("id=\"pasteurl\"") {
            let after_id = &text[id_pos..];

            if let Some(href_pos) = after_id.find("href=\"") {
                let after_href = &after_id[href_pos + "href=\"".len()..];
                if let Some(end) = after_href.find('"') {
                    let url = &after_href[..end];
                    if url.starts_with("https://") || url.starts_with("http://") {
                        return Ok(url.to_string());
                    }
                }
            }

            if let Some(href_pos) = after_id.find("href='") {
                let after_href = &after_id[href_pos + "href='".len()..];
                if let Some(end) = after_href.find('\'') {
                    let url = &after_href[..end];
                    if url.starts_with("https://") || url.starts_with("http://") {
                        return Ok(url.to_string());
                    }
                }
            }
        }

        Err(PasteError::InvalidData)
    }

    // 1) try YOURLS proxy first
    match try_method(opts, long_url, "shortenviayourls") {
        Ok(u) => Ok(u),
        Err(e1) => {
            log::debug!("YOURLS shorten failed ({e1:?}), trying shlink…");

            // 2) fall back to shlink proxy
            match try_method(opts, long_url, "shortenviashlink") {
                Ok(u) => Ok(u),
                Err(e2) => {
                    // preserve useful debug, but return a single error
                    log::debug!("Shlink shorten also failed ({e2:?})");
                    Err(PasteError::InvalidData)
                }
            }
        }
    }
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
    let long_url = res.to_paste_url().to_string();

    let should_shorten = opts.shorten && !opts.no_shorten;

    let short_url = if should_shorten {
        match shorten_via_privatebin(opts, &long_url) {
            Ok(s) => Some(s),
            Err(e) => {
                log::debug!("shorten failed, falling back to long URL: {e:?}");

                // User-facing notice (stderr so stdout remains just the URL)
                eprintln!(
                    "--shorten specified but no URL shortener was found configured on host, falling back to host-provided URL."
                );

                None
            }
        }
    } else {
        None
    };

    if opts.json {
        let mut output: Value = serde_json::to_value(res.clone())?;
        output["pasteurl"] = Value::String(long_url.clone());
        output["deleteurl"] = Value::String(res.to_delete_url().to_string());
        if let Some(s) = &short_url {
            output["shorturl"] = Value::String(s.clone());
        }
        std::io::stdout().write_all(serde_json::to_string_pretty(&output)?.as_bytes())?;
    } else {
        let to_print = short_url.as_deref().unwrap_or(&long_url);
        std::io::stdout().write_all(to_print.as_bytes())?;
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

fn handle_scrape(opts: &Opts) -> PbResult<()> {
    let url = opts.get_url();
    let api = API::new(url.clone(), opts.clone());
    let expiries = api.scrape_expiries()?;
    std::io::stdout().write_all(format!("{:?}", expiries).as_bytes())?;
    writeln!(std::io::stdout())?;
    Ok(())
}

fn main() -> PbResult<()> {
    crate::logger::SimpleLogger::init()?;

    if pbcli::config::has_debug_flag() {
        log::set_max_level(log::LevelFilter::Debug);
    }

    let config_args = pbcli::config::get_config_args(pbcli::config::has_skip_default_config_flag());
    let mut env_args = pbcli::config::get_cli_args();
    let mut merged_args: Vec<OsString> = vec![];
    merged_args.extend(env_args.drain(0..1));
    merged_args.extend(config_args);
    merged_args.extend(env_args);

    let opts: Opts = Opts::parse_from(&merged_args);

    if opts.scrape_expiries {
        return handle_scrape(&opts);
    }

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
