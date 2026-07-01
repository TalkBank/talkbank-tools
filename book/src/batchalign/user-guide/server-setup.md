# Server and Fleet Setup

**Status:** Current
**Last updated:** 2026-06-30 13:55 EDT

## Overview

Batchalign3 can run as a persistent server that accepts jobs from remote
clients. This is useful when multiple people share one powerful machine, when
you want warm workers to survive across commands, or when audio files live on a
central server instead of on each laptop.

## Architecture

```text
┌───────────────────────────────────────┐
│  Server machine (GPU, lots of RAM)    │
│                                       │
│  ┌───────────────────────────────┐    │
│  │ batchalign3 server (port 8001)│    │
│  └───────────────────────────────┘    │
│                  │                    │
│             Python workers            │
│          (Stanza, Whisper, etc.)      │
└───────────────────────────────────────┘
         ▲                ▲
    Laptop A          Desktop B
    --server URL      --server URL
```

Clients use `--server http://server:8001` to send work. The server dispatches
to Python workers, manages job lifecycle, and returns results.

## Single-machine server

### 1. Install batchalign3 on the server

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/TalkBank/talkbank-tools/releases/latest/download/install-batchalign3.sh | sh
```

### 2. Configure batchalign3

Create `~/.batchalign3/server.yaml`:

```yaml
port: 8001
host: "0.0.0.0"
max_concurrent_jobs: 4
warmup_commands: [morphotag, align, transcribe]

# Map data repository names to media file locations.
media_mappings:
  my-corpus: /path/to/audio/files
  another-corpus: /path/to/more/audio
```

### 3. Start the server

```bash
batchalign3 serve start --port 8001 --host 0.0.0.0 -v
```

### 4. Connect from clients

On any machine that can reach the server:

```bash
batchalign3 --server http://server:8001 morphotag corpus/ -o output/
batchalign3 --server http://server:8001 align corpus/ -o output/
```

## Shared media via NFS or mounted storage

For audio commands (`align`, `transcribe`), the execution host must be able to
read the media files.

Recommended approach:

1. Export or mount the media directories on the server at a canonical path.
2. Configure `media_mappings` so corpus-relative roots resolve to that mounted storage.
3. Run remote submissions against the server that can already see those paths.

## Server management

```bash
# Check server status
batchalign3 serve status

# Stop the server
batchalign3 serve stop

# View server health
curl http://localhost:8001/health | python3 -m json.tool
```

## Direct mode vs server mode

| Aspect | Direct mode | Server mode |
|--------|------------|-------------|
| Setup | None | `server.yaml` + `batchalign3 serve` |
| Model loading | ~4-7s on first run | Warm workers can be reused |
| Crash recovery | None (restart manually) | SQLite-backed recovery requeues resumable work on next server start |
| Multi-user | No | Yes (concurrent jobs) |
| Remote audio | Must be local | Via shared storage / `media_mappings` |
| Monitoring | Terminal output | Web dashboard |

**Most users should start with direct mode.** Server mode is for teams managing
shared infrastructure.
