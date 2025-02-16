use crate::PasteFormat;
use clap::Parser;
use parse_size::parse_size;
use url::Url;

const ABOUT: &str = "pbcli is a command line client which allows to upload and download
pastes from privatebin directly from the command line.

Project home page: https://github.com/Mydayyy/pbcli";

#[derive(Debug, Parser, Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[clap( version = env ! ("CARGO_PKG_VERSION"), author = "Mydayyy <dev@mydayyy.eu>", about = ABOUT)]
#[clap(term_width(if let Some((terminal_size::Width(w), _)) = terminal_size::terminal_size() { w as usize } else { 120 }))]
#[clap(rename_all = "kebab-case")]
#[command(args_override_self = true)]
pub struct Opts {
    #[clap(required_unless_present("host"))]
    pub url: Option<Url>,

    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    #[clap(long)]
    pub host: Option<Url>,

    #[clap(long, short = 'f', value_enum, default_value = "plaintext")]
    pub format: PasteFormat,

    #[cfg_attr(feature = "uniffi", uniffi(default = "1week"))]
    #[clap(long, short = 'e', default_value = "1week")]
    pub expire: String,

    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    #[clap(long)]
    #[arg(value_parser = |x: &str| parse_size(x))] // closure: https://github.com/clap-rs/clap/issues/4939
    #[clap(help(
        "Prompt if the paste exceeds the given size. Fail in non-interactive environments."
    ))]
    pub size_limit: Option<u64>,

    #[cfg_attr(feature = "uniffi", uniffi(default = false))]
    #[clap(long, help("richer output: for delete_url, comments, etc"))]
    pub json: bool,
    #[cfg_attr(feature = "uniffi", uniffi(default = false))]
    #[clap(long, short = 'b', conflicts_with = "discussion")]
    #[clap(help("enable burn on read for new paste"))]
    pub burn: bool,
    #[cfg_attr(feature = "uniffi", uniffi(default = false))]
    #[clap(long, short = 'd')]
    #[clap(help("enable discussion for new paste"))]
    pub discussion: bool,

    #[cfg_attr(feature = "uniffi", uniffi(default = false))]
    #[clap(long, requires("url"))]
    #[clap(help("make new comment on existing paste"))]
    pub comment: bool,
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    #[clap(long, requires("comment"), value_name = "nickname")]
    #[clap(help("use this nick for comment"))]
    pub comment_as: Option<String>,
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    #[clap(long, requires("comment"), value_name = "parentid")]
    #[clap(help("reply to this parent comment"))]
    pub comment_to: Option<String>,

    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    #[clap(long, value_name = "FILE")]
    pub download: Option<std::path::PathBuf>,
    #[cfg_attr(feature = "uniffi", uniffi(default = false))]
    #[clap(long)]
    #[clap(help("overwrite the file given with --download if it already exists"))]
    pub overwrite: bool,

    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    #[clap(long, value_name = "FILE")]
    pub upload: Option<std::path::PathBuf>,

    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    #[clap(long, short = 'p')]
    pub password: Option<String>,

    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    #[clap(long, requires_all(& ["oidc_client_id", "oidc_username", "oidc_password"]))]
    #[clap(help("oidc token endpoint from which to obtain an access token"))]
    pub oidc_token_url: Option<String>,

    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    #[clap(long)]
    #[clap(help("client id to send to the token endpoint"))]
    pub oidc_client_id: Option<String>,

    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    #[clap(long)]
    #[clap(help("username to send to the token endpoint"))]
    pub oidc_username: Option<String>,

    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    #[clap(long)]
    #[clap(help("password to send to the token endpoint"))]
    pub oidc_password: Option<String>,

    #[cfg_attr(feature = "uniffi", uniffi(default = false))]
    #[clap(long)]
    #[clap(help("print debug output to stderr"))]
    pub debug: bool,

    #[cfg_attr(feature = "uniffi", uniffi(default = false))]
    #[clap(long)]
    #[clap(help("do not look for config in default locations"))]
    pub no_default_config: bool,

    #[cfg_attr(feature = "uniffi", uniffi(default = false))]
    #[clap(long)]
    #[clap(help("attempt scraping supported expiries of given host and exit"))]
    pub scrape_expiries: bool,
}

impl Opts {
    pub fn get_url(&self) -> &Url {
        self.url
            .as_ref()
            .unwrap_or_else(|| self.host.as_ref().unwrap())
    }
}
