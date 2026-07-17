"""Device selection helpers for CPU/GPU/MPS compute backends.

Controls whether batchalign engines use hardware accelerators (CUDA, MPS) or
fall back to CPU.  Runtime callers should prefer the typed
``DevicePolicy(force_cpu=...)`` boundary and keep any environment reads at the
process edge.

Typical usage from the CLI layer::

    if ctx.params["force_cpu"]:
        apply_force_cpu()
"""

from __future__ import annotations

from collections.abc import Mapping, MutableMapping
from dataclasses import dataclass
import os
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import torch as _torch


@dataclass(frozen=True, slots=True)
class DevicePolicy:
    """Typed device preference resolved once at the runtime boundary.

    ``allow_mps`` is the explicit Apple-GPU opt-in
    (``BATCHALIGN_ALLOW_MPS=1``). MPS failures proved RARE but
    catastrophic in production (the 2026-04-05 AGX kernel deadlock hard-
    stalled machines; fp16 corruption incidents in Feb/Mar 2026), so
    CPU remains the safe default and MPS is never selected implicitly.
    Opting in gets fp32 model dtypes on every loader (the non-CUDA
    branches) and never affects engines with MPS correctness bugs (the
    speaker stage's Pyannote emits wrong timestamps on MPS, upstream
    wontfix).
    """

    force_cpu: bool = False
    allow_mps: bool = False

    @classmethod
    def from_environ(cls, environ: Mapping[str, str] | None = None) -> DevicePolicy:
        """Build a policy from an environment mapping."""
        env = environ if environ is not None else os.environ
        return cls(
            force_cpu=env.get("BATCHALIGN_FORCE_CPU") == "1",
            allow_mps=env.get("BATCHALIGN_ALLOW_MPS") == "1",
        )


def apply_force_cpu(environ: MutableMapping[str, str] | None = None) -> DevicePolicy:
    """Set the ``BATCHALIGN_FORCE_CPU`` environment variable to ``"1"``.

    Call this early in the process (before any engine is instantiated) to
    force all subsequent engines onto CPU.  The flag is inherited by child
    processes spawned via ``ProcessPoolExecutor``.
    """
    env = environ if environ is not None else os.environ
    env["BATCHALIGN_FORCE_CPU"] = "1"
    return DevicePolicy(force_cpu=True)


def resolve_inference_device(
    device_policy: DevicePolicy | None = None,
) -> _torch.device:
    """Resolve the concrete PyTorch device for ML model loading.

    The selection order is: CUDA > MPS (explicit opt-in only) > CPU.
    Without ``allow_mps``, MPS is excluded and model loading resolves to
    either CUDA or CPU; see ``DevicePolicy`` for the opt-in rationale
    and history.

    Parameters
    ----------
    device_policy:
        Typed device preference. ``None`` reads ``BATCHALIGN_FORCE_CPU`` from
        the environment via ``DevicePolicy.from_environ()``.

    Returns
    -------
    torch.device
        ``torch.device("cuda")``, ``torch.device("mps")`` (opt-in only),
        or ``torch.device("cpu")``.
    """
    import torch

    resolved_policy = device_policy or DevicePolicy.from_environ()
    if resolved_policy.force_cpu:
        return torch.device("cpu")
    if torch.cuda.is_available():
        return torch.device("cuda")
    if resolved_policy.allow_mps and torch.backends.mps.is_available():
        _warn_mps_engaged()
        return torch.device("mps")
    return torch.device("cpu")


_MPS_WARNING_EMITTED = False


def _warn_mps_engaged() -> None:
    """One warning per process when the MPS opt-in actually engages.

    Honest-risk surface (time-transparency sibling): the user chose the
    fast path; remind them what the rare failure looks like so a
    machine stall is not a mystery.
    """
    global _MPS_WARNING_EMITTED
    if _MPS_WARNING_EMITTED:
        return
    _MPS_WARNING_EMITTED = True
    import logging

    logging.getLogger(__name__).warning(
        "BATCHALIGN_ALLOW_MPS=1: using the Apple GPU. Rare Apple driver "
        "deadlocks have hard-stalled machines under sustained GPU load "
        "(Apr 2026); CPU remains the safe default. Model dtypes stay "
        "float32 on MPS."
    )
