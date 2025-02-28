Privatebin CLI
------------
pbcli is a command line client which allows to upload and download
pastes from privatebin directly from the command line.

It is dual licensed under MIT or the [UNLICENSE](https://unlicense.org).

### Table of Contents

* [Installation](#Installation)
* [Building](#Building)
* [User Guide](#User-Guide)
* [Configuration Files](#Configuration-File)
* [Roadmap](#Roadmap)

### Installation

You can find pre-compiled binaries under the release tab in github.

For Archlinux, it is [available inside the AUR](https://aur.archlinux.org/packages/pbcli/).

### Building

pbcli is written in Rust, so you will need a rust toolchain to
build it.

```
git clone https://github.com/Mydayyy/pbcli.git
cd pbcli
cargo build --release
./target/release/pbcli --version
```

### User Guide

pbcli is simple to use and only involves a few flags. The only
needed argument is either the positional argument `URL` or the option flag
`host`. If both are provided the positional argument URL takes
precedence. To avoid specifying the host / url everytime you can
take advantage of a config file as described [here](#Configuration-File).

When posting a paste you can specify `--json` to receive post details. The output
includes the base58 encoded key used to encrypt/decrypt the paste.
Constructed paste url (including key) and delete url (including token) are also provided for convenience.

Example output:

```json
{
  "baseurl": "https://privatebin.net/",
  "bs58key": "GN3qty1kAFbsGi9FbKKXigXwux1eofhiZQXNVFRMrNQd",
  "deletetoken": "8536f6f8310ed4a9aae0e111b1763f5851cdbefe4c35e4b96bd690269635354a",
  "deleteurl": "https://privatebin.net/?pasteid=31e2e7b19481fa7d&deletetoken=8536f6f8310ed4a9aae0e111b1763f5851cdbefe4c35e4b96bd690269635354a",
  "id": "31e2e7b19481fa7d",
  "pasteurl": "https://privatebin.net/?31e2e7b19481fa7d#GN3qty1kAFbsGi9FbKKXigXwux1eofhiZQXNVFRMrNQd",
  "status": 0,
  "url": "/?31e2e7b19481fa7d"
}
```

---

#### Example usages to get a paste:

```
pbcli https://privatebin.net/?f37ca34e72e2ef77#G8wFGVnpSb4pogzGbMMcgbDgeYkQ5cfcWkcsVwTQJmzd
```

```
pbcli --host https://privatebin.net/?f37ca34e72e2ef77#G8wFGVnpSb4pogzGbMMcgbDgeYkQ5cfcWkcsVwTQJmzd
```

---

#### Example usages to post a new poste

```
echo 'TestPaste' | pbcli --host https://privatebin.net/
```

```
echo 'TestPaste' | pbcli https://privatebin.net/ --json
```

```
echo 'TestPaste' | pbcli --host https://privatebin.net/ --expire=1hour
```

```
echo '## Title\nSome Markdown' | pbcli https://privatebin.net/ --format markdown
```

```
echo 'TestPaste' | pbcli --host https://privatebin.net/ --burn
```

---

#### CLI Help:

```
pbcli is a command line client which allows to upload and download
pastes from privatebin directly from the command line.

Project home page: https://github.com/Mydayyy/pbcli

Usage: pbcli [OPTIONS] [URL]

Arguments:
  [URL]  

Options:
      --host <HOST>                      
  -f, --format <FORMAT>                  [default: plaintext] [possible values: plaintext, syntax, markdown]
  -e, --expire <EXPIRE>                  [default: 1week]
      --size-limit <SIZE_LIMIT>          Prompt if the paste exceeds the given size. Fail in non-interactive environments.
      --json                             richer output: for delete_url, comments, etc
  -b, --burn                             enable burn on read for new paste
  -B, --no-burn                          disable burn if set
  -d, --discussion                       enable discussion for new paste
  -D, --no-discussion                    disable burn if set
      --comment                          make new comment on existing paste
      --comment-as <nickname>            use this nick for comment
      --comment-to <parentid>            reply to this parent comment
  -o, --download <FILE>                  
  -w, --overwrite                        overwrite the file given with --download if it already exists
  -u, --upload <FILE>                    
  -p, --password <PASSWORD>              
      --oidc-token-url <OIDC_TOKEN_URL>  oidc token endpoint from which to obtain an access token
      --oidc-client-id <OIDC_CLIENT_ID>  client id to send to the token endpoint
      --oidc-username <OIDC_USERNAME>    username to send to the token endpoint
      --oidc-password <OIDC_PASSWORD>    password to send to the token endpoint
      --debug                            print debug output to stderr
      --no-default-config                do not look for config in default locations
      --scrape-expiries                  attempt scraping supported expiries of given host and exit
  -h, --help                             Print help
  -V, --version                          Print version
```

### Configuration File

pbcli supports a configuration file to fine tune the default behaviour.
You need to set the environment variable `PBCLI_CONFIG_PATH`  to a file path. The file
needs to contain flags you want to pass to pbcli, one per line.
Lines starting with a # are ignored. An useful case for this may be
setting a default instance to use by setting the --host argument.

Additionally, pbcli will look in a few default directories to find a config. This can be disabled
via the `--no-default-config` argument. The directories it checks are:

- `$XDG_CONFIG_HOME/pbcli/config` if the XDG variable is set. Otherwise `$HOME/.config/pbcli/config`
- /etc/pbcli/config

As well as the windows and mac equivalents to those linux paths. Details can be found
here: https://docs.rs/directories/latest/directories/struct.ProjectDirs.html#method.config_local_dir

The debug mode can be used to troubleshoot config loading issues. It will print the used config as well
as the directories that were searched for a config.

Instead of typing `echo 'test' | pbcli https://privatebin.net/` you'll only need
to type `echo 'test' | pbcli`

Example config:

```
--host=https://privatebin.net/
--expire=1month
```

### Uniffi

This projects offers uniffi bindings. In order to enable them,
build the library with the uniffi feature enabled.
You can learn more about uniffi [here](https://github.com/mozilla/uniffi-rs).
Additionally to see an example integration of pbcli with uniffi
enabled into an android project you can check out [sharepaste](https://github.com/nain-F49FF806/sharepaste.oo).

### Roadmap

- Descriptive error messages
- Add support for auth mechanism
    - Basic auth
    - ~~oauth~~ Added in v2.2.0 using Resource Owner Password Credential Grant flow
- ~~Add support for file attachments~~ Added in v2.1.0

### Credits

- [nain](https://github.com/nain-F49FF806) for the uniffi implementation
