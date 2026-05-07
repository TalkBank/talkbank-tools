"""Regression: progress events emitted before the ready handshake must
not corrupt the supervisor's first-line read.

The Rust supervisor reads exactly one JSON line from worker stdout
during the handshake — the ready signal
``{"ready": true, "pid": N, "transport": ...}``. Anything else on the
first line fails the parse with
"invalid ready JSON: missing field `ready`".

This had concrete consequences in production: a worker that needed to
download a Stanza catalog or HuggingFace model during bootstrap would
fire an ``emit_download_event`` *before* ``_print_ready()`` ran. Without
the gate this test pins, the resulting ``progress_v2`` JSON line would
beat the ready signal out the door and every per-language file in the
batch failed with a ready-parse error.

The fix gates ``write_progress_event`` on a module-level flag in
``_protocol`` (``_handshake_complete``). Pre-ready emissions reroute to
stderr as plain log lines; post-ready emissions go to stdout normally.
``_print_ready`` and ``_print_ready_tcp`` flip the flag the moment the
ready line is on the wire.
"""

from __future__ import annotations

import io
import json
import sys
from unittest import mock

from batchalign.worker import _protocol


def _reset_handshake_state():
    """Restore the module-level handshake flag for test isolation."""
    _protocol._handshake_complete = False


def test_pre_ready_progress_event_goes_to_stderr():
    """Before ready, ``write_progress_event`` must not write to stdout.

    The supervisor's first-line read is reserved for the ready envelope;
    a stray JSON line breaks the handshake and fails every job in the
    batch with "invalid ready JSON: missing field `ready`".
    """
    _reset_handshake_state()
    fake_stdout = io.StringIO()
    fake_stderr = io.StringIO()
    with (
        mock.patch.object(sys, "stdout", fake_stdout),
        mock.patch.object(sys, "stderr", fake_stderr),
    ):
        _protocol.write_progress_event(
            request_id="",
            completed=0,
            total=0,
            stage="downloading_stanza_catalog",
        )

    assert fake_stdout.getvalue() == "", (
        "Pre-ready progress events must NOT touch stdout — that's "
        "reserved for the ready envelope. Got: "
        f"{fake_stdout.getvalue()!r}"
    )
    stderr_text = fake_stderr.getvalue()
    assert "downloading_stanza_catalog" in stderr_text, (
        "Pre-ready progress events should still log to stderr so the "
        "operator can see what the worker is doing during bootstrap. "
        f"Got: {stderr_text!r}"
    )


def test_print_ready_flips_the_flag_and_writes_ready_envelope():
    """``_print_ready()`` writes the ready envelope to stdout and flips the flag.

    After this point, progress events go to stdout normally.
    """
    _reset_handshake_state()
    fake_stdout = io.StringIO()
    fake_stderr = io.StringIO()
    with (
        mock.patch.object(sys, "stdout", fake_stdout),
        mock.patch.object(sys, "stderr", fake_stderr),
    ):
        _protocol._print_ready()

    line = fake_stdout.getvalue().strip()
    envelope = json.loads(line)
    assert envelope["ready"] is True
    assert envelope["transport"] == "stdio"
    assert "pid" in envelope
    assert _protocol._handshake_complete is True


def test_post_ready_progress_event_goes_to_stdout():
    """After ``_print_ready()``, progress events use the normal stdout path.

    This is the contract that lets the runner's ``spawn_progress_forwarder``
    multiplex progress events into the per-job status sink.
    """
    _reset_handshake_state()
    # Manually flip the flag (skip the ready-write so the test isolates
    # post-ready behavior).
    _protocol._handshake_complete = True
    try:
        fake_stdout = io.StringIO()
        fake_stderr = io.StringIO()
        with (
            mock.patch.object(sys, "stdout", fake_stdout),
            mock.patch.object(sys, "stderr", fake_stderr),
        ):
            _protocol.write_progress_event(
                request_id="req-7",
                completed=0,
                total=0,
                stage="downloading_hf_openai_whisper-large-v3",
            )

        line = fake_stdout.getvalue().strip()
        envelope = json.loads(line)
        assert envelope["op"] == "progress_v2"
        assert envelope["event"]["request_id"] == "req-7"
        assert envelope["event"]["stage"] == "downloading_hf_openai_whisper-large-v3"
        # Post-ready should NOT also log to stderr — that would double-
        # report the same event.
        assert fake_stderr.getvalue() == ""
    finally:
        _reset_handshake_state()


def test_print_ready_tcp_flips_the_flag_via_stderr_route():
    """``_print_ready_tcp()`` flips the flag even though ready goes to stderr.

    TCP-mode workers signal readiness on stderr (the CLI launcher reads
    it there); stdout is unused for the handshake. We still flip the
    flag so any code path that calls ``write_progress_event`` post-bind
    behaves consistently across transports.
    """
    _reset_handshake_state()
    fake_stdout = io.StringIO()
    fake_stderr = io.StringIO()
    with (
        mock.patch.object(sys, "stdout", fake_stdout),
        mock.patch.object(sys, "stderr", fake_stderr),
    ):
        _protocol._print_ready_tcp("127.0.0.1", 9100)

    # Ready line went to stderr.
    line = fake_stderr.getvalue().strip()
    envelope = json.loads(line)
    assert envelope["ready"] is True
    assert envelope["transport"] == "tcp"
    assert envelope["port"] == 9100
    # Stdout untouched.
    assert fake_stdout.getvalue() == ""
    # Flag flipped.
    assert _protocol._handshake_complete is True
    _reset_handshake_state()
