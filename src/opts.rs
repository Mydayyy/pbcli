use crate::PasteFormat;
use clap::Parser;
use parse_size::parse_size;
use url::Url;

const ABOUT: &str = "pbcli is a command line client which allows to upload and download
pastes from privatebin directly from the command line.

Project home page: https://github.com/Mydayyy/pbcli";

#[derive(Debug, Parser, Clone, uniffi::Record)]
#[clap(setting = clap::AppSettings::AllArgsOverrideSelf, version = env ! ("CARGO_PKG_VERSION"), author = "Mydayyy <dev@mydayyy.eu>", about = ABOUT)]
#[clap(term_width(if let Some((terminal_size::Width(w), _)) = terminal_size::terminal_size() { w as usize } else { 120 }))]
#[clap(rename_all = "kebab-case")]
pub struct Opts {
    #[clap(required_unless_present("host"), parse(try_from_str))]
    pub url: Option<Url>,

    #[uniffi(default = None)]
    #[clap(long, parse(try_from_str))]
    pub host: Option<Url>,

    #[clap(long, arg_enum, default_value = "plaintext")]
    pub format: PasteFormat,

    #[uniffi(default = "1week")]
    #[clap(long, default_value = "1week")]
    pub expire: String,

    #[uniffi(default = None)]
    #[clap(long, parse(try_from_str = parse_size))]
    #[clap(help(
        "Prompt if the paste exceeds the given size. Fail in non-interactive environments."
    ))]
    pub size_limit: Option<u64>,

    #[uniffi(default = false)]
    #[clap(long, help("richer output: for delete_url, comments, etc"))]
    pub json: bool,
    #[uniffi(default = false)]
    #[clap(long, conflicts_with = "discussion")]
    #[clap(help("enable burn on read for new paste"))]
    pub burn: bool,
    #[uniffi(default = false)]
    #[clap(long)]
    #[clap(help("enable discussion for new paste"))]
    pub discussion: bool,

    #[clap(long, requires("url"))]
    #[clap(help("make new comment on existing paste"))]
    pub comment: bool,
    #[clap(long, requires("comment"), value_name = "nickname")]
    #[clap(help("use this nick for comment"))]
    pub comment_as: Option<String>,
    #[clap(long, requires("comment"), value_name = "parentid")]
    #[clap(help("reply to this parent comment"))]
    pub comment_to: Option<String>,

    #[uniffi(default = None)]
    #[clap(long, parse(from_os_str), value_name = "FILE")]
    pub download: Option<std::path::PathBuf>,
    #[uniffi(default = false)]
    #[clap(long)]
    #[clap(help("overwrite the file given with --download if it already exists"))]
    pub overwrite: bool,

    #[uniffi(default = None)]
    #[clap(long, parse(from_os_str), value_name = "FILE")]
    pub upload: Option<std::path::PathBuf>,

    #[uniffi(default = None)]
    #[clap(long)]
    pub password: Option<String>,

    #[uniffi(default = None)]
    #[clap(long, requires_all(& ["oidc-client-id", "oidc-username", "oidc-password"]))]
    #[clap(help("oidc token endpoint from which to obtain an access token"))]
    pub oidc_token_url: Option<String>,

    #[uniffi(default = None)]
    #[clap(long)]
    #[clap(help("client id to send to the token endpoint"))]
    pub oidc_client_id: Option<String>,

    #[uniffi(default = None)]
    #[clap(long)]
    #[clap(help("username to send to the token endpoint"))]
    pub oidc_username: Option<String>,

    #[uniffi(default = None)]
    #[clap(long)]
    #[clap(help("password to send to the token endpoint"))]
    pub oidc_password: Option<String>,
}

impl Opts {
    pub fn get_url(&self) -> &Url {
        self.url
            .as_ref()
            .unwrap_or_else(|| self.host.as_ref().unwrap())
    }
}
