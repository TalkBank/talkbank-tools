# Server Mode

**Status:** Current
**Last updated:** 2026-06-30 13:55 EDT

Batchalign includes a built-in HTTP server managed by `batchalign3 serve ...`.
Ordinary local processing commands can still run inline, but when
`auto_daemon: true` (the default) the CLI first tries to reuse or start a
loopback daemon so warm workers survive across commands. `--no-server` and
`--sequential` still force direct local execution.

## Current routing rules

- With `--server URL`, the CLI submits supported jobs to that server in content mode.
- `transcribe`, `transcribe_s`, `benchmark`, and `avqi` prefer the local daemon when `auto_daemon` is enabled.
- Without an explicit remote target, `auto_daemon: true` makes the CLI reuse or start a loopback daemon before it falls back to direct local execution.
- Local-daemon and auto-detected loopback-server paths use shared-filesystem `paths_mode` for local-audio commands such as `align`, `transcribe`, `benchmark`, `opensmile`, and `avqi`.
- Explicit `--server` always stays on content mode, even when the URL is `localhost`.

## Backend model

The server now has a single **local in-process control plane**.

- There is no Temporal backend and no backend-selection config.
- Job detail surfaces still report `control_plane.backend`, but the only released value is `local`.
- On restart, in-flight work from the old process does **not** continue running in place. Recovery reloads queued/interrupted work from SQLite and re-dispatches resumable jobs when the server comes back up.

## Start a server

Foreground:

```bash
batchalign3 serve start --foreground
```

Background:

```bash
batchalign3 serve start
```

Useful flags:

```bash
batchalign3 serve start --foreground --port 8000
batchalign3 serve start --foreground --config ~/server.yaml
batchalign3 serve start --foreground --warmup minimal
batchalign3 serve start --foreground --test-echo
```

## Check and stop a server

```bash
batchalign3 serve status
batchalign3 serve status --server http://myserver:8000
batchalign3 serve stop
```

Inspect remote jobs:

```bash
batchalign3 jobs --server http://myserver:8000
batchalign3 jobs --server http://myserver:8000 <JOB_ID>
```

## Server configuration

Default config path:

```text
~/.batchalign3/server.yaml
```

Minimal example:

```yaml
default_lang: eng
port: 8000
max_concurrent_jobs: 8
auto_daemon: true
warmup_commands: [morphotag, align, transcribe]
media_roots: []
media_mappings: {}
```

`warmup_commands` marks commands that are *eligible* for warmup. The current
production startup path remains lazy by default, so this key is not a promise
that those workers will preload on every boot.

Important keys:

- `port`: server listen port
- `host`: bind address (defaults to `0.0.0.0`)
- `max_concurrent_jobs`: `0` means auto-tune
- `auto_daemon`: reuse or start a loopback daemon for ordinary CLI processing
- `warmup_commands`: list of commands eligible for warmup
- `media_roots`: local execution-host media lookup roots
- `media_mappings`: local execution-host root mappings from corpus paths to mounted media paths
- `memory_tier`: override auto-detected tier: `small`, `medium`, `large`, `fleet`
- `memory_gate_mb`: host headroom reserve in MB (default: 2048)
- `gpu_startup_mb` / `stanza_startup_mb` / `io_startup_mb`: per-profile startup reservation overrides
- `worker_health_interval_s`: health check frequency in seconds (default: 30)
- `job_ttl_days`: auto-delete completed jobs after this many days (default: 7)

OTLP tracing can be enabled by setting `BATCHALIGN_OTLP_ENDPOINT`
(or `OTEL_EXPORTER_OTLP_ENDPOINT`) in the server environment.

`server.yaml` uses a strict schema. Unknown keys are rejected at startup
instead of being silently ignored, so stale config must be updated to the
current key set.

## Remote use

Commands that support explicit remote dispatch look like this:

```bash
batchalign3 --server http://myserver:8000 morphotag corpus/ -o output/
batchalign3 --server http://myserver:8000 align corpus/ -o output/
```

For audio commands, `--server` now means "run this on a host that can already
see these filesystem paths." The clean operational model is to run the CLI on
the execution host itself (or to reach it over SSH/VNC) rather than expecting
the server to infer media from a different client machine's directory layout.
When the corpus clone root and the mounted media root differ on that execution
host, use local `media_mappings` or `--media-dir` as explicit root replacement.
