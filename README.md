pbcli
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
`host`.  If both are provided the positional argument URL takes
precedence. To avoid specifying the host / url everytime you can 
take advantage of a config file as described [here](#Configuration-File).

When posting a paste you can specify `--json` to receive post details. The output
includes the base58 encoded key used to encrypt/decrypt the paste
and can be used to construct the paste url.

Example output:
```
{"deletetoken":"ajae8c36aa945ff93a04bef4ff08fa505f96d49e1z28eb09a36l797c2eaeg952",
"id":"e6a227cfbc0fec3e",
"status":0,
"url":"/?e6a227cfbc0fec3e",
"bs58key":"31rvVHezWQH7sh7tgZGxfQJGKK4WLLCwFBL64Jr5nhLu"}
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
pbcli 2.2.0

Mydayyy <dev@mydayyy.eu>

pbcli is a command line client which allows to upload and download
pastes from privatebin directly from the command line.

Project home page: https://github.com/Mydayyy/pbcli

USAGE:
    pbcli [OPTIONS] [URL]

ARGS:
    <URL>    

OPTIONS:
        --burn                               
        --discussion                         
        --download <FILE>                    
        --expire <EXPIRE>                    [default: 1week]
        --format <FORMAT>                    [default: plaintext] [possible values: plaintext, syntax, markdown]
    -h, --help                               Print help information
        --host <HOST>                        
        --json                               
        --oidc-client-id <OIDC_CLIENT_ID>    
        --oidc-password <OIDC_PASSWORD>      
        --oidc-token-url <OIDC_TOKEN_URL>    
        --oidc-username <OIDC_USERNAME>      
        --overwrite                          
        --password <PASSWORD>                
        --upload <FILE>                      
    -V, --version                            Print version information
```

### Configuration File

pbcli supports a configuration file to fine tune the default behaviour.
You need to set the environment variable `PBCLI_CONFIG_PATH`  to a file path. The file 
needs to contain flags you want to pass to pbcli, one per line.
Lines starting with a # are ignored. An useful case for this may be
setting a default instance to use by setting the --host argument.

Instead of typing `echo 'test' | pbcli https://privatebin.net/` you'll only need
to type `echo 'test' | pbcli`

Example config:
```
--host=https://privatebin.net/
--expire=1month
```

### Roadmap

- Descriptive error messages
- Add support for auth mechanism 
  - Basic auth
  - ~~oauth~~ Added in v2.2.0 using Resource Owner Password Credential Grant flow
- ~~Add support for file attachments~~ Added in v2.1.0