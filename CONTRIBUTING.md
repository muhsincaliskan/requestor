# Contributing to Requestor

Thanks for helping. Bug reports, ideas and pull requests are all welcome.

## Reporting a bug

Open an issue with the Requestor version (shown in Settings), your operating system, what you did and what you expected. If a collection or request triggers it, attach a small example with tokens and passwords removed.

## Building

Requestor needs a recent stable Rust toolchain (see `rust-version` in `Cargo.toml`). The first command starts the desktop app, the second prints the command-line help.

```
cargo run -p requestor-app
cargo run -p requestor-cli -- --help
```

## Before opening a pull request

Run the same checks as CI:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Please keep these in mind:

- **Memory comes first.** A change that adds noticeable memory use needs a reason. `scripts/measure-ram.sh` measures the scenarios CI checks.
- **No new dependencies without discussion.** Open an issue first, and list every added crate in the pull request.
- **Postman compatibility.** Collections and environments must keep round-tripping through the Postman v2.1 formats, including fields Requestor does not understand.
- **Secrets stay secret.** Never log tokens, passwords or full payloads, and keep secret values masked in the UI.
- **Small, focused changes.** One topic per pull request, with tests for changed behavior.

`CLAUDE.md` lists the architecture rules the codebase follows, such as keeping behavior on the types that own the data.

## License

By contributing you agree that your contributions are licensed under the [MIT License](LICENSE).
