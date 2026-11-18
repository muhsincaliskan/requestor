# Requestor: handoff for the remaining roadmap

Repo: github.com/muhsincaliskan/requestor (Rust workspace: crates/core, crates/cli, crates/app with egui).
Read CLAUDE.md first. Its rules are strict:
- Domain types own behaviour.
- No nested types.
- Use named constants.
- No explanatory comments.
- Cognitive complexity ≤ 10 (clippy enforces this).
- Modules stay around 200 lines and are split before 250.
- No new dependencies without the owner's approval.

Checks to run before every PR:
```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
GitHub Actions has no minutes left, so run these locally.

## State on 2026-11-19 (updated 00:30 Istanbul)

- main is at PR #49. PR #48 added the accent colour; PR #49 added the JSONPath filter and the Tree view and is merged, with the review fixes applied.
- The released version is v0.3.0. It does not include PRs #44–#49.
- Branch `claude/project-thread-k9h1og` equals main. Work only on that branch.
- Small open item from item 7: the filter text and the Tree/Pretty layout reset on every send. To fix it, carry both over in `state/sending.rs:153`, the same way `with_previous` carries the previous response.

## Remaining roadmap

Each item is one PR. The owner lets PRs merge without asking once the checks pass.

- **Item 6: response timing breakdown** (DNS, connect plus TLS, first byte, download).
  - APPROVED by the owner (2026-11-18 21:10Z): add `tower-service` and `tower-layer` as direct dependencies. Both are already in Cargo.lock through reqwest 0.13.5.
  - Plan:
    - Build the reqwest blocking `ClientBuilder` with `dns_resolver(Arc<impl Resolve>)` to time DNS.
    - Use `connector_layer(...)` to time the connect step (TCP plus TLS).
    - Relate the timings to the request by authority and start time, because clients are cached and shared (see `ReqwestTransport` in `crates/core/src/transport`).
    - A reused connection shows no DNS or connect time.
    - Show the breakdown in a hover on the elapsed time in `response_viewer/status.rs`.
  - Without approval: show DNS (resolver only), first byte (when `send()` returns) and download (body read).
- **Item 8: collection runner in the app. Mostly done, NOT merged.**
  - Code is on branch `claude/project-thread-k9h1og`, commit 23b1d66 "WIP: run collections from the app". fmt, clippy and all 747 tests pass.
  - Works: sidebar "Run collection…", a dialog with order (↑/↓), enable toggles, iterations and delay, a results table, Stop, a "Run anyway" confirmation for protected environments, an error line when the transport cannot be built, and cancelling the in-flight request. The pause happens only between steps.
  - Main files: `crates/core/src/collection/runner.rs` (`RunSession`, `RunPacing`, `RunStepResult`), `crates/app/src/state/collection_run.rs`, `crates/app/src/runner/collection_run.rs`, `crates/app/src/panels/overlays/collection_runner.rs`.
  - Reviewer findings still open:
    - (4) Variables extracted during a run are not saved back. A single send stores them through `workspace.store_extracted` in `state/sending.rs`.
    - (6) The CLI `Session` in `crates/cli/src/run.rs` duplicates `RunSession`. Let the CLI use `RunSession`, passing extra variables.
    - (7) Only closing the dialog stops a run. Stop the run in state whenever the overlay stops being `CollectionRunner`.
    - (11) Results grid: use `ScrollArea::show_rows` for long runs.
    - (12) More tests: a worker loop with a fake transport, the confirmation flow, the failure message.
  - Before the PR, do a visual check under Xvfb, then open a PR titled "Run collections from the app" and squash merge it.
- **Release 0.4.0.**
  - Bump versions.
  - Linux and Windows: CI is out of minutes, so build them locally.
  - macOS: build on the owner's Mac with `scripts/bundle-macos.sh` and upload with `gh release`.
  - `scripts/install.sh` only works once the repo is public.
