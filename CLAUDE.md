# Requestor

A lightweight desktop and command-line HTTP client in Rust. The primary goal is a small memory footprint; the second is a friendly UI. Collections and environments stay compatible with the Postman v2.1 JSON formats.

## Workspace

- `crates/core`: HTTP engine, collection and environment models, variable resolution, importers and exporters. No UI or terminal dependencies.
- `crates/cli`: standalone command-line binary built on `core`.
- `crates/app`: egui desktop application built on `core`.

## Architecture rules

- Domain types own their behavior. Normalization, validation, deduplication, state transitions and calculations live on the type that owns the data, never in a service.
- Conversions belong to the types: a request type offers `from(domain)`, a response type offers `to_domain()`, a public response offers `from(domain)`. No private `to_*`, `convert_*` or `map_*` helpers in services.
- Services only orchestrate: call clients, invoke domain behavior, handle errors, compose the result.
- Domain types never depend on transport, UI or framework types.
- Keep the established boundary for external calls: transport client, application-facing client, and an implementation that orchestrates them.
- No hard-coded values that are repeated, configurable or carry domain meaning. Use named constants or configuration.
- No explanatory comments when naming can carry the meaning.
- Prefer small focused changes. Do not refactor unrelated code.
- No new dependencies without explicit approval from the project owner. List every dependency a change adds in the pull request.
- Cognitive complexity of any function stays at or below 10, enforced by `clippy.toml`. Modules stay near 200 lines; split when a module approaches 250.
- Unknown fields in imported documents are preserved so that export never silently drops data.
- Never log secrets, tokens or full payloads. Secret values are masked in the UI.

## Commands

- Format check: `cargo fmt --all -- --check`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Test: `cargo test --workspace`
- Single crate: `cargo test -p requestor-core`
- Memory measurement on macOS: `scripts/measure-ram.sh`

## Working with the team of agents

- Foundation and shared types are built serially by the main session. Independent modules (importers, UI panels) may be delegated to builder agents running in separate worktrees.
- The reviewer agent audits each finished step against the rules above before the pull request is opened.
- The researcher agent handles crate, format and documentation lookups.
- Consult the advisor before committing to a plan, when the same error recurs, and before declaring a step done.

## Platform order

macOS first. Linux and Windows follow when requested.
