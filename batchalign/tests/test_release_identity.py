"""Release-identity regression tests.

The wheel version is canonical.  The small runtime metadata file is retained
for compatibility, so it must not be allowed to describe a different release.
"""

from importlib.metadata import version

from batchalign.runtime import VERSION_NUMBER


def test_runtime_metadata_matches_installed_distribution() -> None:
    """The packaged compatibility metadata must identify this wheel."""
    assert VERSION_NUMBER == version("batchalign3")
