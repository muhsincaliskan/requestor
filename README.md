# Requestor

A lightweight HTTP request client written in Rust, built to keep memory use small.

## Layout

- `crates/core`: collection and environment models (Postman v2.1 compatible), HTTP method handling.
- `crates/cli`: command-line binary (`requestor`), a stub for now.
- `crates/app`: egui desktop application. Currently a memory-measurement spike: a docked request and response view with a virtualized, syntax-highlighted response viewer.

## Development

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Measuring memory

```
cargo build --release -p requestor-app
scripts/measure-ram.sh 5 8
```

The first argument is the synthetic response size in megabytes, the second is the settle time in seconds. On macOS the script prints `footprint` output for the process (the physical footprint is what Activity Monitor reports) next to `ps` RSS. To compare with Postman, add up every Postman process (main, Helper renderer and GPU) and compare the total with Requestor's single process.
