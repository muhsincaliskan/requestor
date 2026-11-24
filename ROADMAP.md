# Roadmap

Ideas and planned work. Nothing here is available until it appears in the [README](README.md).

## Planned: request scripts

A small English script language, run by Requestor itself with no JavaScript engine.

- Rename the request's **Tests** tab to **Scripts**, with **Before request** and **After response** sections.
- One statement per line: `set`, `expect`, `save`, `fail`, and `if` / `else if` / `else` / `end`. No functions or open-ended loops.
- Convert imported Postman scripts when every line is recognized. Otherwise keep the original unchanged, do not run it, and show which lines were not recognized.
- Export every script as Postman JavaScript. An imported script that was not edited is exported exactly as it came.
- A read-only JavaScript view next to the script.
- Describe the language in the README in the same change that adds it.

Later: `run "Request"` with a retry, and signing statements such as HMAC.

## Considering: JavaScript as an advanced option

Run full Postman scripts in an embedded QuickJS engine, for scripts the language above cannot express. It adds a dependency, so it needs the owner's approval.

- Start the engine only while a script runs, with time and memory limits and no file or network access.
- Never run an imported script before the user allows it.

Measured on 2026-11-24 with `rquickjs` 0.14.0 in a small standalone release program on Apple silicon, as physical footprint. The `pm` API is not included.

| Measurement | Result |
| --- | --- |
| Binary size | +1.0 MB |
| One engine running a script | +0.2 MB, not returned after closing |
| Start, run and close 1000 times | 0.09 ms each on average, +0.5 MB at the end |
| Parse a 4.8 MB JSON body into JavaScript objects | +32 MB, about 8 MB kept after closing |
