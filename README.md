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
- Import from Postman, OpenAPI 3 (JSON), WSDL 1.1, cURL and raw HTTP requests. Paste a cURL command into the URL field to fill in the whole request.
- A WSDL becomes a collection with one ready SOAP request per operation: the address, the SOAPAction (or the SOAP 1.2 action), and a sample envelope with `?` where values go. Each SOAP port gets its own folder. Schemas and WSDLs it imports are read from next to the file. Paste a WSDL address such as `https://host/service.asmx?wsdl` into the import window to download it together with the documents it imports; pasted WSDL text cannot load them.
- Variables:
  - `{{variables}}` resolve from the collection and the active environment.
  - Fields suggest names as you type and color each placeholder: defined in blue, undefined in red.
- SOAP requests:
  - The SOAP body type starts from a 1.1 or 1.2 envelope and has a SOAPAction field.
  - It sends the matching Content-Type.
  - It is stored as raw XML, so Postman opens it unchanged.
- GraphQL: **Fetch schema** asks the server for its schema with the request's URL, headers and auth. The schema's queries and mutations are listed beside the query editor with a filter. Click a field to replace the query with a ready operation that takes its arguments as variables and selects its simple fields. Values already filled in the variables are kept. The schema is kept while the tab is open.
- Tests on each request:
  - Checks for status, response time, body text, a header or a JSON value.
  - Save a JSON value or a header into an environment or collection variable for the next requests.
  - Stored as Postman test scripts, so the same collection runs in Postman. Custom scripts are kept as they are.
- Secret values are masked in the UI and never logged.
- Secret environment values and the proxy password are kept in the system keychain (macOS Keychain, Windows Credential Manager, the Secret Service on Linux), one entry per environment. The workspace files and every export leave them empty, so a command-line run that reads a workspace environment file sees them empty too. When no keychain is available, or it refuses a value, the values stay in the workspace files and Requestor says so. Secret collection and folder variables are left empty in exports.
- Large responses scroll smoothly:
  - The body view is virtualized and syntax highlighted.
  - It has Pretty and Raw modes for JSON and XML, and a find box.
- Headers and cookies have one-click copy.
- **Generate code** (in the **…** menu next to Save) turns the request into cURL, an `.http` file, JavaScript `fetch`, Python `requests` or Java `HttpClient` code. Secret values show as `<redacted>` unless **Include secrets** is ticked. The last language you picked is remembered. Files the request refers to do not have to exist on this computer. Header values outside ASCII are sent as UTF-8 bytes in the JavaScript and Python code; Java's `HttpClient` accepts only ASCII header values and rejects or replaces the rest.
- Send a request again and the Changes tab shows what differs from the previous response, line by line, with JSON and XML laid out first.
- Server-sent events (`text/event-stream`) show up in an Events tab as they arrive, with each event's type, id and data. Press **Stop** to end the stream and keep what came in. Click an event to copy its data. The last 1000 events are listed. A stream ends at 16 MB, or at the response size limit if that is lower, and stops with a notice when the server stays quiet for longer than the request timeout.
- WebSocket: a `ws://` or `wss://` URL turns **Send** into **Connect**. Messages in both directions are listed live with their time. Write a message under the list and press **Send** or Cmd+Enter. Binary messages show their size, and clicking a text message copies it. Headers and auth from the request go into the handshake. WebSocket connections cannot use a proxy yet: with a proxy set, only hosts in its bypass list connect. They always verify TLS certificates and do not send stored cookies. The message list keeps the last 1000 messages, up to about 16 MB.
- The History section in the sidebar lists the last 100 requests you sent, with their status. Click one to open it in a new tab that still uses its collection's variables and auth. Sending the same request twice in a row keeps one entry. Request bodies over 32 KB are left out, and responses are never kept. History saves requests as written, including header and auth values, so keep tokens in environment variables. Right-click the History heading to clear it.
- On start, Requestor asks GitHub once whether a newer release exists, and if so shows a link to its download page in the status bar. Turn it off with **Check for updates** in Settings.
- A URL typed without `http://` or `https://` is sent over HTTP, as in a browser.
- With autosave off, requests with unsaved edits are marked in the tree and listed in an Unsaved group at the top of the sidebar.
- A standalone `requestor` command-line binary runs collections, sends single cURL or raw HTTP requests, and converts imports to Postman format.

## Keeping a collection in git

Right-click a collection and choose **Save to folder…** to keep it in a folder you pick, for example inside your project's repository. Every request becomes its own small JSON file, so changes show up as clear diffs and merge cleanly:

```
api/
  collection.json        name, variables, auth and the order of the items
  health.json            one file per request
  orders/
    folder.json          the folder's settings and the order of its requests
    list-orders.json
```

- File names come from the request names. Requests a teammate adds as new files are picked up on the next start.
- Only files Requestor wrote are ever changed or removed. Other files, such as a README or a `package.json`, are left alone: a request whose name matches one is saved as `name-2.json` instead. Linked directories are not followed.
- Values are saved as written, so keep tokens and passwords in an environment rather than in the collection you commit.
- **Open collection folder…** in the import window opens a folder someone else saved.
- **Stop saving to folder** keeps the files and moves the collection back into the app's own storage. Deleting the collection in the app also leaves the folder alone.
- `requestor run api/` runs a saved folder just like a collection file.

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
| Generate code | Cmd+Option+G |
| Settings | Cmd+, |
| Toggle sidebar | Cmd+B |
| Toggle two-pane view | Cmd+Shift+L |
| Show middle workbench | Cmd+Option+M |
| Swap left and right sidebars | Cmd+Option+S |

## Command line

```
requestor run collection.json --env dev.json --folder Pets
requestor run api/ --env dev.json
requestor send "curl -X POST https://example.test/pets -d '{\"name\":\"Rex\"}'"
requestor import openapi.json --to postman
requestor import service.wsdl --to postman
requestor import "https://host/service.asmx?wsdl" --to postman
```

`run` evaluates the checks saved with each request and passes values extracted from a response to the requests after it. A failed check fails the request.

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
