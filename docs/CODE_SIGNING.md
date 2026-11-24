# Code signing policy

Windows builds of Requestor (`requestor-app.exe`) are signed.
Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org).

Every signed file is built from the source code in this repository by the [release workflow](../.github/workflows/release.yml) on GitHub-hosted runners, and every signing request is approved by hand.

## Team roles

- Committers and reviewers: [Muhsin Caliskan](https://github.com/muhsincaliskan). Pull requests from anyone else are reviewed by a committer before they are merged.
- Approvers: [Muhsin Caliskan](https://github.com/muhsincaliskan), the repository owner.

All team members use multi-factor authentication on GitHub and SignPath.

## Privacy policy

This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it.

The one automatic connection is the update check: when the app starts it asks github.com for the latest release of Requestor. It sends no data about you or your requests. Turn it off in Settings › About › Check for updates.

Requests you send go only to the addresses you enter.
