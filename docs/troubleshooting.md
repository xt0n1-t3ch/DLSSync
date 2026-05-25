# Troubleshooting and diagnostics

DLSSync writes a rolling log file on every launch and can open a pre-filled GitHub
issue with that log attached. Use this when an update fails, a scan comes up empty,
or anything behaves unexpectedly.

## Where the logs live

Logs are written to a per-user folder, one file per day:

```
%USERPROFILE%\DLSSync\Logs\dlssync.log.<YYYY-MM-DD>
```

The current file is `dlssync.log` plus a date suffix. Older days stay on disk until
you remove them; nothing is uploaded anywhere on its own.

Open the folder from **About → Logs** (the `Open logs folder` button), or from the
command palette. Each line carries a timestamp, level, target module, and message —
for example a failed apply records the stage that broke and the classified error.

## Reporting a problem

Two entry points build the same report:

- **About → Report a problem** — for general issues.
- **Apply progress modal → Report issue** — shown when an apply has a failed feature;
  it also attaches the per-file apply report (stages, errors, timings).

Both open a GitHub issue in your browser, pre-filled with:

- DLSSync version
- OS name and build
- timestamp
- the last 40 log lines (collapsed in a `<details>` block)
- the optional context (your description, or the apply report)

Nothing is sent automatically. The browser opens the GitHub "new issue" form so you
review and edit everything before submitting. The report body is capped so the URL
stays within GitHub's limit; the full log is still in the logs folder if more detail
is needed.

## Log levels

The default filter is `info` for the app and its workspace crates. To capture more
detail for a single run, set `RUST_LOG` before launching, e.g.:

```
RUST_LOG=dlssync=debug,dll_catalog=debug
```

## Common cases

- **An update failed** — open the apply modal's `Report issue`, or read the latest log
  file. The error class (network, signature, hash, lock, permission, backup) is logged
  next to the file that failed.
- **A backup's `Reveal snapshot file` opens the wrong place** — the snapshot folder is
  opened with the file selected; if the snapshot was deleted outside DLSSync, its parent
  folder opens instead.
- **The logs folder is empty** — it is created on first launch; relaunch once if you
  installed and immediately opened it.
