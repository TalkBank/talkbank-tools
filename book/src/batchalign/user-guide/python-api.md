# No Python API

**Status:** Current
**Last updated:** 2026-05-01 22:47 EDT

Batchalign3 does not have a public Python API. Python lives inside the
package as a worker-side ML inference layer — strictly an internal
implementation detail of the Rust runtime. As of 2026-05, only Rev.AI
ASR is Rust-owned (driven directly from the server); every other ASR
engine, plus all morphosyntactic / segmentation / translation / coref
pipelines, runs through a Python worker. The long-term direction is to
keep narrowing the Python layer as Rust gains coverage of more ML
pieces, but no Whisper-in-Rust path is shipping today.

## The CLI is the entry point

All processing is done through the `batchalign3` command-line tool:

```bash
batchalign3 transcribe input/ -o output/ --lang eng
batchalign3 morphotag input/ -o output/ --lang eng
batchalign3 align input/ -o output/ --lang eng
```

For programmatic use from Python, call the CLI as a subprocess:

```python
import subprocess

subprocess.run(
    [
        "batchalign3", "morphotag",
        "input/", "-o", "output/",
        "--lang", "eng",
    ],
    check=True,
)
```

Anything else — importing `batchalign.*` modules, calling
`batchalign_core.*` symbols, depending on `batchalign.providers`,
`batchalign.worker.*`, or any Python class or function — is unsupported
and will break without notice. There is no compatibility surface to
build against.

## If you used the BA2 Python API

The following BA2 Python entry points were removed during the BA3
rewrite. The CLI is the replacement for all of them:

| BA2 Python entry point | Replacement |
| --- | --- |
| `BatchalignPipeline` | `batchalign3 <command>` |
| `WhisperEngine`, `RevAIEngine`, etc. | `batchalign3 transcribe --asr-engine <name>` |
| `CHATFile`, `Document`, `ParsedChat` | the `chatter` CLI in talkbank-tools (`chatter to-json`, `chatter validate`, etc.) |
| `run_pipeline()`, `LocalProviderInvoker`, `PipelineOperation` | `batchalign3 <command>` |
| `compute_wer()` | `batchalign3 compare` |
| `batchalign.compat` | none — its purpose was to bridge BA2 callers; rewrite around the CLI |

If you have BA2 Python integration code, port it to subprocess calls
into `batchalign3`. The CLI's output format is the long-term stable
contract.

## See also

- [CLI Reference](cli-reference.md)
- [Migration: User Workflow](../migration/user-migration.md)
