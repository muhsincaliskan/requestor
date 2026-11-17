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

## Features

- Collections and environments stay compatible with Postman v2.1 JSON. Fields Requestor does not know are kept on export.
- Import from Postman, OpenAPI 3 (JSON), cURL and raw HTTP requests. Paste a cURL command into the URL field to fill in the whole request.
- Variables:
  - `{{variables}}` resolve from the collection and the active environment.
  - Fields suggest names as you type and color each placeholder: defined in blue, undefined in red.
- SOAP requests:
  - The SOAP body type starts from a 1.1 or 1.2 envelope and has a SOAPAction field.
  - It sends the matching Content-Type.
  - It is stored as raw XML, so Postman opens it unchanged.
- Secret values are masked in the UI and never logged.
- Large responses scroll smoothly:
  - The body view is virtualized and syntax highlighted.
  - It has Pretty and Raw modes for JSON and XML, and a find box.
- Headers and cookies have one-click copy.
- With autosave off, requests with unsaved edits are marked in the tree and listed in an Unsaved group at the top of the sidebar.
- A standalone `requestor` command-line binary runs collections, sends single cURL or raw HTTP requests, and converts imports to Postman format.

## Keyboard shortcuts

`Cmd` is `Ctrl` and `Option` is `Alt` on Linux and Windows.

| Action | Keys |
| --- | --- |
| Command palette: run a command, open a request, switch environment or theme | Cmd+K |
| Send request | Cmd+Enter |
| Save request | Cmd+S |
| New, close, duplicate tab | Cmd+T, Cmd+W, Cmd+D |
| Next, previous tab | Ctrl+Tab, Ctrl+Shift+Tab |
| Find in response | Cmd+F |
| Import | Cmd+O |
| Settings | Cmd+, |
| Toggle sidebar | Cmd+B |
| Toggle two-pane view | Cmd+Shift+L |
| Show middle workbench | Cmd+Option+M |
| Swap left and right sidebars | Cmd+Option+S |

## Command line

```
requestor run collection.json --env dev.json --folder Pets
requestor send "curl -X POST https://example.test/pets -d '{\"name\":\"Rex\"}'"
requestor import openapi.json --to postman
```

Run `requestor --help` for every option. The exit code is 0 when every request succeeded, 1 when one failed and 2 for usage or input errors.

## Layout

- `crates/core`: HTTP engine, collection and environment models, variable resolution, importers and exporters. No UI dependencies.
- `crates/cli`: the `requestor` command-line binary.
- `crates/app`: the egui desktop application.

## Development

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Measuring memory

```
scripts/measure-ram.sh 5 0 120
scripts/measure-ram.sh 8 5 180
```

The arguments are the number of open tabs, the size of the response loaded into the active tab in megabytes, and the budget in megabytes. A footprint over the budget is reported as a warning; over 200 MB the script fails. On macOS it reports the physical footprint, which is what Activity Monitor shows. To compare with Postman, add up every Postman process (main, Helper renderer and GPU) and compare the total with Requestor's single process.

## Fonts

The interface uses [Inter](https://github.com/rsms/inter) in Regular, Medium and SemiBold, reduced to the Latin and interface symbol ranges. Inter is licensed under the SIL Open Font License 1.1; see `assets/fonts/LICENSE-Inter.txt`.
