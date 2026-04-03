# rust-librdb

Rust wrapper for [librdb](https://github.com/redis/librdb), the Redis RDB file parser.

Parses RDB dump files and delivers Redis data types (strings, lists, hashes,
sets, sorted sets, streams) through a callback trait, without loading the
entire file into memory.

| Crate | Description |
|-------|-------------|
| [`librdb`](librdb/) | Safe, high-level wrapper |
| [`librdb-sys`](librdb-sys/) | Raw FFI bindings |

## Installation

```toml
[dependencies]
librdb = "0.1"
```

By default, librdb is compiled from the vendored source (git submodule).
A C compiler is required.

```sh
git clone --recurse-submodules https://github.com/funcpp/rust-librdb
cd rust-librdb
cargo build
```

### Linking options

| Method | How |
|--------|-----|
| Source build (default) | No extra steps |
| System shared library | `--features librdb-sys/dynamic-linking` |
| Pre-built static library | Set `DEP_LIBRDB_STATIC_ROOT`, use `--features librdb-sys/static-linking` |

## Example

```rust
use librdb::{Parser, RdbHandlers, KeyInfo, Result};

struct MyHandler {
    keys: Vec<String>,
}

impl RdbHandlers for MyHandler {
    fn handle_new_key(&mut self, key: &[u8], _info: &KeyInfo) -> Result<()> {
        self.keys.push(String::from_utf8_lossy(key).into_owned());
        Ok(())
    }
}

fn main() -> librdb::Result<()> {
    let mut parser = Parser::new(MyHandler { keys: vec![] })?;
    parser.parse_file("dump.rdb")?;
    let handler = parser.into_handler();
    println!("{} keys parsed", handler.keys.len());
    Ok(())
}
```

`&[u8]` callback parameters borrow directly from librdb's internal buffer and
are only valid for the duration of the callback. Clone if you need to retain
the data.

## License

MIT
