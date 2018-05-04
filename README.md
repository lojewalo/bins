# bins

*A tool for pasting from the terminal.*

[![Travis](https://img.shields.io/travis/jkcclemens/bins/master.svg)](https://travis-ci.org/jkcclemens/bins)
[![Crates.io](https://img.shields.io/crates/v/bins.svg)](https://crates.io/crates/bins)
[![Crates.io](https://img.shields.io/crates/d/bins.svg)](https://crates.io/crates/bins)
[![license](https://img.shields.io/github/license/jkcclemens/bins.svg)](https://github.com/jkcclemens/bins/blob/master/LICENSE)

Supports [GitHub Gist](https://gist.github.com/), [Pastebin](http://pastebin.com/), [hastebin](http://hastebin.com/),
[sprunge](http://sprunge.us/), [Bitbucket snippets](https://bitbucket.org/snippets/),
[fedora pastebin](https://paste.fedoraproject.org/) and [paste.gg](https://paste.gg).

---

## Install

bins is built with the latest Rust nightly. Other versions can be used, but your mileage may vary.

### Release

#### No Rust

Don't want to install Rust? A precompiled binary may be available for your architecture at the
[latest release](https://github.com/jkcclemens/bins/releases/latest).

#### Rust

If you want to install the [latest release](https://crates.io/crates/bins) from [crates.io](https://crates.io/):

```sh
# If you don't have Rust installed:
# curl https://sh.rustup.rs -sSf | sh
cargo install bins
```

### Development

**Building from source requires the nightly compiler!**

#### Requirements

Depending on the features of bins you have enabled, there are different requirements for building bins from source.

- openssl
  - libssl-dev
- clipboard_support
  - xorg-dev on Linux
- file_type_checking
  - libmagic-dev
- rustls
  - No requirements

If you want to install the latest version from the repository:

```sh
git clone https://github.com/jkcclemens/bins
cd bins
# If you don't have Rust installed:
# curl https://sh.rustup.rs -sSf | sh
cargo install
```

Add `$HOME/.cargo/bin` to your `$PATH` or move `$HOME/.cargo/bin/bins` to `/usr/local/bin`.

## Upgrade

To upgrade an existing installation from crates.io:

```
cargo install --force bins
```

To upgrade an existing installation from source:

```
cd bins
git fetch origin && git reset --hard origin/master
cargo install --force
```

## Usage

To get help, use `bins -h`. bins accepts a list of multiple files, a string, or piped data.

Take a look at some of the written examples below:

### Examples

#### Creating a paste from stdin

```shell
$ echo "testing123" | bins -b gist
https://gist.github.com/fa772739e946eefdd082547ed1ec9d2c
```

#### Creating pastes from files

Pasting a single file:

```
$ bins -b gist hello.c
https://gist.github.com/215883b109a0047fe07f5ee229de6a51
```

bins supports pasting multiple files, too. With services such as GitHub's [gist](https://gist.github.com), these are
natively supported. For services which don't support multiple file pastes, an index paste is created and returned which
links to individual pastes for each file.

```
$ bins -b gist hello.c goodbye.c
https://gist.github.com/anonymous/7348da5d3f1cd8134d7cd6ee1cf5e84d
```

#### Specifying visibility options

By default, bins will use the `defaults.private` option from the config file to determine whether or not to create a
private paste. The default value of this is `true` - so new pastes will be private for a fresh install. You can override
this at the command line:

```
$ bins --public --bin gist hello.c
https://gist.github.com/05285845622e5d6164f0d36b73685b19
```

### Configuration

Running bins at least once will generate a configuration file. Its location is dependent on the environment that bins is
run in. The configuration file will be created at the first available location in the list below:

- `$XDG_CONFIG_DIR/bins.cfg`
- `$HOME/.config/bins.cfg`
- `$HOME/.bins.cfg`

If none of these paths are available (`$XDG_CONFIG_DIR` and `$HOME` are either both unset or unwritable), bins will fail
and not generate a config file.

The configuration file is documented when it is generated, so check the file for configuration documentation.
