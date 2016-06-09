# bins

*A tool for pasting from the terminal.*

 Supports [GitHub Gist](https://gist.github.com/), [Pastebin](http://pastebin.com/), [Pastie](http://pastie.org), and [Hastebin](http://hastebin.com/).

---

## Install

**bins requires at least Rust 1.8.0.**

```sh
git clone https://github.com/jkcclemens/bins
cd bins
# If you don't have Rust installed:
# curl -sSf https://static.rust-lang.org/rustup.sh | sh
cargo install
```

Add `$HOME/.cargo/bin` to your `$PATH` or move `$HOME/.cargo/bin/bins` to `/usr/local/bin`.

## Upgrade

To upgrade an existing installation:

```
cd bins
git fetch origin && git reset --hard origin/master
cargo uninstall bins
cargo install
```

## Video demo

[![](https://asciinema.org/a/48288.png)](https://asciinema.org/a/48288)

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

bins supports pasting multiple files, too. With services such as GitHub's [gist](https://gist.github.com), these are natively supported. For services which don't support multiple file pastes, an index paste is created and returned which links to individual pastes for each file.

```
$ bins -s gist hello.c goodbye.c 
https://gist.github.com/anonymous/7348da5d3f1cd8134d7cd6ee1cf5e84d
```

```
$ bins -s pastie hello.c goodbye.c
http://pastie.org/private/v9enoe4qbxgh6ivlazxmaa
```

#### Specifying visibility options

By default, bins will use the `defaults.private` option from the config file to determine whether or not to create a private paste. The default value of this is `true` - so new pastes will be private for a fresh install. You can override this at the command line:

```
$ bins --public -s gist hello.c 
https://gist.github.com/05285845622e5d6164f0d36b73685b19
```

### Configuration

There is a configuration file with documentation that is generated at `$HOME/.bins.cfg` after the first run of the
program.
