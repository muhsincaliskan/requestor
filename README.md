<p align="center"><img src="assets/icon/requestor-512.png" width="128" alt="Requestor icon"></p>

# Requestor

A lightweight HTTP request client written in Rust, built to keep memory use small.

## Install

Download the latest build from the Releases page: a `.dmg` or zipped `.app` for macOS (universal), a `.tar.gz` for Linux or a `.zip` for Windows. On macOS open the `.dmg` and drag Requestor to Applications.

The macOS app is only ad-hoc signed, so on first launch Gatekeeper may block it. Right-click the app and choose Open, or run:

```
xattr -dr com.apple.quarantine /Applications/Requestor.app
```

The `requestor` command-line binary is attached to the same release; run `requestor --help` for usage.

To build the macOS app locally:

```
scripts/bundle-macos.sh
```

This writes `target/bundle/Requestor.app` and, when `hdiutil` is available, `target/bundle/Requestor-<version>.dmg`. Pass both `aarch64-apple-darwin` and `x86_64-apple-darwin` for a universal build.

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
