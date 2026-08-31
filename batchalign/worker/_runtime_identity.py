"""Content identity for the Python code that executes worker requests.

The ready handshake exposes digests, never local filesystem paths.  A process
computes the identity once; all later observers receive the same immutable
value rather than re-reading a potentially changed installation.
"""

from __future__ import annotations

import hashlib
import importlib
import importlib.machinery
import importlib.metadata
import sys
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path
from types import ModuleType
from typing import Protocol


class DigestWriter(Protocol):
    """Minimal streaming digest operation used by admitted package files."""

    def update(self, value: bytes) -> None: ...


@dataclass(frozen=True)
class Sha256Digest:
    """Lowercase SHA-256 text constructed only from observed bytes."""

    value: str

    def __post_init__(self) -> None:
        if len(self.value) != 64 or any(
            character not in "0123456789abcdef" for character in self.value
        ):
            raise ValueError(
                "runtime digest must be 64 lowercase SHA-256 hexadecimal characters"
            )

    @classmethod
    def of_bytes(cls, value: bytes) -> Sha256Digest:
        return cls(hashlib.sha256(value).hexdigest())


@dataclass(frozen=True)
class WorkerRuntimeIdentity:
    """Path-free content evidence for one executing Python environment."""

    python_version: str
    python_executable_sha256: Sha256Digest
    batchalign_package_tree_sha256: Sha256Digest
    batchalign_core_extension_sha256: Sha256Digest
    distribution_inventory_sha256: Sha256Digest

    def json_value(self) -> dict[str, int | str]:
        return {
            "schema_version": 1,
            "python_version": self.python_version,
            "python_executable_sha256": self.python_executable_sha256.value,
            "batchalign_package_tree_sha256": self.batchalign_package_tree_sha256.value,
            "batchalign_core_extension_sha256": (
                self.batchalign_core_extension_sha256.value
            ),
            "distribution_inventory_sha256": self.distribution_inventory_sha256.value,
        }


class RuntimeIdentityError(RuntimeError):
    """The executing runtime could not acquire a stable content identity."""


@dataclass(frozen=True)
class LoadedNativeExtension:
    """Filesystem-backed native extension proven to be loaded in this process."""

    path: Path

    @classmethod
    def from_module(cls, module: ModuleType) -> LoadedNativeExtension:
        """Admit only an existing file with a recognized extension-module suffix."""

        module_file = getattr(module, "__file__", None)
        if not isinstance(module_file, str):
            raise RuntimeIdentityError(
                "loaded batchalign_core extension has no filesystem identity"
            )
        path = Path(module_file).resolve()
        if not path.is_file() or not any(
            path.name.endswith(suffix)
            for suffix in importlib.machinery.EXTENSION_SUFFIXES
        ):
            raise RuntimeIdentityError(
                "loaded batchalign_core module is not a native extension file"
            )
        return cls(path)

    def sha256(self) -> Sha256Digest:
        """Hash the exact native code loaded by the Python interpreter."""

        return _hash_file(self.path)


@dataclass(frozen=True)
class RuntimePackageFile:
    """One admitted runtime file paired with its stable relative identity."""

    path: Path
    relative: bytes

    @classmethod
    def admit(cls, root: Path, path: Path) -> RuntimePackageFile | None:
        relative_path = path.relative_to(root)
        parts = relative_path.parts
        if (
            not path.is_file()
            or path.suffix == ".pyc"
            or "__pycache__" in parts
            or (parts and parts[0] == "tests")
            or any(part.startswith(".") for part in parts)
        ):
            return None
        return cls(path=path, relative=relative_path.as_posix().encode())

    def contribute_to(self, digest: DigestWriter) -> None:
        digest.update(len(self.relative).to_bytes(8, "big"))
        digest.update(self.relative)
        content_digest = hashlib.sha256()
        try:
            with self.path.open("rb") as source:
                for chunk in iter(lambda: source.read(1024 * 1024), b""):
                    content_digest.update(chunk)
        except OSError as error:
            raise RuntimeIdentityError(
                "batchalign runtime package changed while its identity was observed"
            ) from error
        # A fixed-width per-file digest makes the tree encoding unambiguous:
        # file contents cannot impersonate the frame for a following path.
        digest.update(content_digest.digest())


def _hash_file(path: Path) -> Sha256Digest:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise RuntimeIdentityError(
            "runtime artifact changed while its identity was observed"
        ) from error
    return Sha256Digest(digest.hexdigest())


def _hash_package_tree(root: Path) -> Sha256Digest:
    """Hash admitted runtime paths and bytes, excluding generated/test files."""

    digest = hashlib.sha256()
    files = tuple(
        admitted
        for path in sorted(root.rglob("*"))
        if (admitted := RuntimePackageFile.admit(root, path)) is not None
    )
    if not files:
        raise RuntimeIdentityError("batchalign runtime package tree is empty")
    for package_file in files:
        package_file.contribute_to(digest)
    return Sha256Digest(digest.hexdigest())


@lru_cache(maxsize=1)
def observe_worker_runtime() -> WorkerRuntimeIdentity:
    """Observe and freeze the executing runtime identity for this process."""

    import batchalign

    core_extension = LoadedNativeExtension.from_module(
        importlib.import_module("batchalign_core.batchalign_core")
    )
    package_file = getattr(batchalign, "__file__", None)
    if not isinstance(package_file, str):
        raise RuntimeIdentityError(
            "loaded batchalign package has no filesystem identity"
        )
    distributions = tuple(
        sorted(
            f"{distribution.metadata.get('Name', '<unnamed>')}=={distribution.version}"
            for distribution in importlib.metadata.distributions()
        )
    )
    distribution_bytes = ("\n".join(distributions) + "\n").encode()
    return WorkerRuntimeIdentity(
        python_version=".".join(str(part) for part in sys.version_info[:3]),
        python_executable_sha256=_hash_file(Path(sys.executable).resolve()),
        batchalign_package_tree_sha256=_hash_package_tree(
            Path(package_file).resolve().parent
        ),
        batchalign_core_extension_sha256=core_extension.sha256(),
        distribution_inventory_sha256=Sha256Digest.of_bytes(distribution_bytes),
    )
