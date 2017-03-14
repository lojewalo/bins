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

## 2.0.0 changes

Hello and welcome to the breaking-change-filled, unstable world of bins 2!

Please delete your config or update it to match the one in the repo root.

### bins supported

Only gist, hastebin, and sprunge have been implemented so far. All of the original bins will be supported when the
rewrite is complete.

### Safety

A new config section has been added: `safety`. Some options have been moved here, and new options have been added.

- `cancel_on_unsupported`: cancel uploads if attempting to use a feature with a bin that does not support it
- `warn_on_unsupported`: print a warning if attempting to use a feature with a bin that does not support it

### Usage

Input mode has been renamed and its option has been removed. If the first positional argument to the program is a valid
URL, bins switches to download mode. Specifying file names after filters the files downloaded.

### Threading

Uploads and downloads are now threaded. When uploading to a bin that does not support multiple files, the files are
uploaded in parallel, waiting for them to complete before generating and uploading the index file.

When downloading multiple files, the downloads are run in parallel. Some bins, such as gist, requires that a request is
made to retrieve the raw URLs, so that request is made, then the threaded requests are made.

Threading is handled with a thread pool initialized with the number of threads ready equal to the number of CPU cores.

### JSON output

JSON output is improved and more standardized. Errors will output in JSON soon (not yet implemented).

### Downloading

Every effort has been made to keep the number of requests made to an absolute minimum. If something is downloaded once,
it is not ever downloaded again (per execution). If you find something that is downloaded more than once, file a bug
report.

Yes, this is an improvement. bins 1 would redownload files at times when checking for index files, etc.

### Internal

The internal API has been reduced and simplified. This makes the implementations for individual bins appear more
complex, but there is less need to hack the functionality to make it fit into the API.

bins can declare features they support, automatically have upload and download methods implemented (sometimes), should
always be able to be `Sync`, work with URLs in a more manageable way, and more.

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

```
$ bins -b pastie hello.c goodbye.c
http://pastie.org/private/v9enoe4qbxgh6ivlazxmaa
```

#### Specifying visibility options

By default, bins will use the `defaults.private` option from the config file to determine whether or not to create a
private paste. The default value of this is `true` - so new pastes will be private for a fresh install. You can override
this at the command line:

```
$ bins --public bs gist hello.c
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
