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

## State on 2026-11-19

- main is at PR #48 (accent colour). The released version is v0.3.0. It does not include PRs #44–#48.
- Branch `claude/project-thread-k9h1og` has an unmerged WIP commit for roadmap item 7 (JSONPath filter + Tree view). Tests pass, but it has the review findings below.

### Item 7 WIP: files

Core:
- `crates/core/src/json_tree.rs` with `read.rs` and `write.rs`: `JsonNode`, which keeps member order and has a custom serde implementation.
- `crates/core/src/json_query.rs` with `cursor.rs`, `selector.rs` and `filter.rs`: the JSONPath parser and evaluator.
- `crates/core/tests/json_query.rs`.

App:
- `crates/app/src/response/inspection.rs`: `BodyLayout` (Pretty/Raw/Tree) and `JsonInspection`.
- `crates/app/src/response/mod.rs`: `ResponseView` now has `layout`, `set_filter`, `json_tree` and `filter_result_count`.
- The comparison methods moved to `response/comparison.rs`.
- `crates/app/src/panels/response_viewer/tree.rs` and `json_filter.rs`, wired into `body.rs`.

### Item 7: review findings to fix before the PR (most severe first)

1. **Memory.**
   - Call `shrink_to_fit()` on `items` and `entries` in `json_tree/read.rs` `visit_seq` and `visit_map`. A 5 MB body currently becomes a 46 MB tree; 27 MB is needed.
   - `inspection.rs` `selected()` deep-copies the matches; `$..*` on 5 MB holds 86 MB. Store the matches as position paths (`Vec<Vec<usize>>`) resolved with `JsonNode::child`. Write the text straight from the matches with `JsonNode::list_text`.
   - The RAM budget is 180 MB for 8 tabs plus a 5 MB response.
2. **Slices with a negative step** (`selector.rs`). Clamp start and end as `if v < 0 { (len+v).max(-1) } else { v.min(len-1) }`, then iterate with `((end+1)..=start).rev().step_by(step.unsigned_abs())`.
   - `$[2:-10:-1]` on `[0,1,2]` must give `[2,1,0]`.
   - `$[-10::-1]` must give `[]`.
3. **Overflow.** `step.abs()` panics on `i64::MIN`. Use `unsigned_abs()`.
4. **Shorthand member names** (`cursor.rs` `name()`). Stop at anything that is not ASCII alphanumeric, `_`, `-` or non-ASCII, so `$[?(@.p<9)]` and `$[?@.t=='a']` work. Add tests for both.
5. **Redraws while typing.** `set_filter` should clear the display only when the parsed `JsonQuery` changes, not on every keystroke. Keep the last valid query.
6. **Tree performance** (`tree.rs`).
   - Nest the chunks (chunks of chunks) so a huge array draws at most 100 headers.
   - Use `id.with(position)` instead of `key_text` for ids. This also avoids clashes from duplicate keys.
   - Truncate `summary()` to a named maximum length.
7. **Rule fixes.**
   - Replace `Option<Option<JsonNode>>` with an enum `NotParsed | Unparseable | Parsed(JsonNode)`.
   - Implement `Display` for `JsonKey` in core.
   - Move `self_and_descendants` to `JsonNode::descendants` and the comparison `order` to a `JsonNode` method.
   - Pass plain parameters instead of the tuple in `tree.rs` `node`.
   - Name the inlined constants: the BodyLayout labels, `'-'` and `"-+.eE"` in `cursor.rs`, the chunk format, and the summary formats.
8. **Toolbar.** In `body.rs`, base the Tree toolbar on whether `json_tree()` returns a tree, so invalid JSON keeps Find and the line count.
9. **Error position.** Add the trimmed leading spaces back to the reported error position.
10. **Quoted strings.** Decode JSON escapes in quoted strings, and require a word boundary after `true`, `false` and `null`.
11. **Optional.** Keep the filter text and layout across sends. Carry them like `with_previous` in `state/sending.rs:153`.

## Remaining roadmap after item 7

Each item is one PR. The owner lets PRs merge without asking once the checks pass.

- **Item 6: response timing breakdown** (DNS, connect plus TLS, first byte, download).
  - Waiting on the owner: approval to add `tower-service` and `tower-layer` as direct dependencies. Both are already in Cargo.lock through reqwest 0.13.5.
  - Plan:
    - Build the reqwest blocking `ClientBuilder` with `dns_resolver(Arc<impl Resolve>)` to time DNS.
    - Use `connector_layer(...)` to time the connect step (TCP plus TLS).
    - Relate the timings to the request by authority and start time, because clients are cached and shared (see `ReqwestTransport` in `crates/core/src/transport`).
    - A reused connection shows no DNS or connect time.
    - Show the breakdown in a hover on the elapsed time in `response_viewer/status.rs`.
  - Without approval: show DNS (resolver only), first byte (when `send()` returns) and download (body read).
- **Item 8: collection runner in the app.**
  - Core already has `runnable_requests` and a CLI runner.
  - Add an overlay with request order (drag or up/down), iterations, delay in ms, and a results table (name, status, time, checks).
  - Run on the existing runner thread.
- **Release 0.4.0.**
  - Bump versions.
  - Linux and Windows: CI is out of minutes, so build them locally.
  - macOS: build on the owner's Mac with `scripts/bundle-macos.sh` and upload with `gh release`.
  - `scripts/install.sh` only works once the repo is public.
