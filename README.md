# bins

*A tool for pasting from the terminal.*

[![Travis](https://img.shields.io/travis/jkcclemens/bins/master.svg)](https://travis-ci.org/jkcclemens/bins)
[![AppVeyor branch](https://img.shields.io/appveyor/ci/jkcclemens/bins/master.svg)](https://ci.appveyor.com/project/jkcclemens/bins)
[![Crates.io](https://img.shields.io/crates/v/bins.svg)](https://crates.io/crates/bins)
[![Crates.io](https://img.shields.io/crates/d/bins.svg)](https://crates.io/crates/bins)
[![license](https://img.shields.io/github/license/jkcclemens/bins.svg)](https://github.com/jkcclemens/bins/blob/master/LICENSE)

 Supports [GitHub Gist](https://gist.github.com/), [Pastebin](http://pastebin.com/), [Pastie](http://pastie.org),
 [Hastebin](http://hastebin.com/), [sprunge](http://sprunge.us/),
 and [Bitbucket snippets](https://bitbucket.org/snippets/).

---

## Install

**bins requires at least Rust 1.10.0.**

### Release

#### No Rust

Don't want to install Rust? A precompiled binary may be available for your architecture at the
[latest release](https://github.com/jkcclemens/bins/releases/latest).

#### Rust

If you want to install the [latest release](https://crates.io/crates/bins) from
[crates.io](https://crates.io/):

```sh
# If you don't have Rust installed:
# curl https://sh.rustup.rs -sSf | sh
cargo install bins
```

### Development

**Building from source requires the beta or nightly compiler!**

This is due to the new `panic = "abort"` option having a few bugs in stable.

If you want to install the latest version from the repository:

```sh
git clone https://github.com/jkcclemens/bins
cd bins
# If you don't have Rust installed:
# curl https://sh.rustup.rs -sSf | sh
cargo install
```

Add `$HOME/.cargo/bin` to your `$PATH` or move `$HOME/.cargo/bin/bins` to `/usr/local/bin`.

### x11/clipboard (Linux-only)

If you are in an environment without `x11`, use `cargo install --no-default-features` to disable clipboard support for
bins. If clipboard support is enabled, which it is by default, your build will fail without `x11`! Note that it's not sufficient to simply have X11 installed, you need the development libraries/headers (provided by `xorg-dev` on Debian systems).

It is also worth noting that the crate used for clipboard support only fills the X clipboard while bins is running, so
unless you are using a clipboard manager, you won't be able to effectively use clipboard support on Linux. If this is
your case, I recommend piping the output of bins to your clipboard, using `xclip` or `xsel` instead.

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

## Video demo

[![](https://asciinema.org/a/50217.png)](https://asciinema.org/a/50217)

The demo is 10 minutes and 12 seconds for the full experience. It starts with a basic overview that ends at 2:08. If you
want the full experience but are strapped for time, there is a
[fast version](https://asciinema.org/a/amvptojaq33mndgqchijekvws) (7:25) and also a
[ludicrous speed version](https://asciinema.org/a/als0avp0zerf1j00gg43hev7c) (4:34).

## Usage

To get help, use `bins -h`. bins accepts a list of multiple files, a string, or piped data.

Take a look at some of the written examples below:

### Examples

#### Creating a paste from stdin

```shell
$ echo "testing123" | bins -s gist
https://gist.github.com/fa772739e946eefdd082547ed1ec9d2c
```

#### Creating pastes from files

Pasting a single file:

```
$ bins -s gist hello.c
https://gist.github.com/215883b109a0047fe07f5ee229de6a51
```

bins supports pasting multiple files, too. With services such as GitHub's [gist](https://gist.github.com), these are
natively supported. For services which don't support multiple file pastes, an index paste is created and returned which
links to individual pastes for each file.

```
$ bins -s gist hello.c goodbye.c 
https://gist.github.com/anonymous/7348da5d3f1cd8134d7cd6ee1cf5e84d
```

```
$ bins -s pastie hello.c goodbye.c
http://pastie.org/private/v9enoe4qbxgh6ivlazxmaa
```

#### Specifying visibility options

By default, bins will use the `defaults.private` option from the config file to determine whether or not to create a
private paste. The default value of this is `true` - so new pastes will be private for a fresh install. You can override
this at the command line:

```
$ bins --public -s gist hello.c 
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
