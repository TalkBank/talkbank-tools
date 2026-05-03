"""Shared fixtures for Cantonese morphosyntax integration tests.

Tests in this directory are Cantonese-specific integration tests
that validate the pipeline end-to-end with Cantonese-specific data and corpus requirements.

These are marked with @pytest.mark.cantonese_integration to allow skipping in fast CI.
The Stanza pipeline fixtures for English (and other languages) are reused from
the parent conftest in morphosyntax/.
"""

from __future__ import annotations

import pytest


def pytest_collection_modifyitems(items):
    """Mark all tests in this directory as Cantonese integration tests."""
    for item in items:
        item.add_marker(pytest.mark.cantonese_integration)
