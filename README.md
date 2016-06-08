# bins

*A tool for pasting from the terminal.*

---

## Install

```sh
git clone https://github.com/jkcclemens/bins
cd bins
# If you don't have Rust installed:
# curl -sSf https://static.rust-lang.org/rustup.sh | sh
cargo install
```

Add `$HOME/.cargo/bin` to your `$PATH` or move `$HOME/.cargo/bin/bins` to `/usr/local/bin`.

## Usage

To get help, use `bins -h`. bins accepts a list of multiple files, a string, or piped data.

See [asciinema](https://asciinema.org/a/48190) for a demo.

### Configuration

There is a configuration file with documentation that is generated at `$HOME/.bins.cfg` after the first run of the
program.
