"""Contracts for classifying Hugging Face Hub access/credential failures."""

from __future__ import annotations

from batchalign.inference._model_access_errors import (
    ModelAccessDeniedError,
    classify_huggingface_access_error,
)


def _response(status_code: int, url: str):
    from requests import Response

    response = Response()
    response.status_code = status_code
    response.url = url
    return response


def test_gated_repo_error_is_classified_as_model_access_denied() -> None:
    from huggingface_hub.errors import GatedRepoError

    error = GatedRepoError(
        "403 Client Error: gated repo",
        response=_response(
            403,
            "https://huggingface.co/pyannote/speaker-diarization-community-1/"
            "resolve/main/plda/xvec_transform.npz",
        ),
    )

    classified = classify_huggingface_access_error(error)

    assert isinstance(classified, ModelAccessDeniedError)
    assert classified.repo_id == "pyannote/speaker-diarization-community-1"
    assert "gated" in str(classified)
    assert "hf auth login" in str(classified)
    assert "hf_token" in str(classified)
    assert "~/.batchalign.ini" in str(classified)
    assert "[auth]" in str(classified)
    assert "pyannote/speaker-diarization-community-1" in str(classified)


def test_repository_not_found_error_is_classified() -> None:
    from huggingface_hub.errors import RepositoryNotFoundError

    error = RepositoryNotFoundError(
        "401 Client Error",
        response=_response(401, "https://huggingface.co/some/private-repo"),
    )

    classified = classify_huggingface_access_error(error)

    assert isinstance(classified, ModelAccessDeniedError)
    assert classified.repo_id == "some/private-repo"


def test_local_entry_not_found_error_is_classified_without_a_repo_id() -> None:
    from huggingface_hub.errors import LocalEntryNotFoundError

    error = LocalEntryNotFoundError("Cannot find the requested files in the disk cache")

    classified = classify_huggingface_access_error(error)

    assert isinstance(classified, ModelAccessDeniedError)
    assert classified.repo_id is None
    assert "hf auth login" in str(classified)


def test_bare_401_http_error_is_classified() -> None:
    from huggingface_hub.errors import HfHubHTTPError

    error = HfHubHTTPError(
        "401 Client Error",
        response=_response(
            401, "https://huggingface.co/owner/model/resolve/main/file.bin"
        ),
    )

    classified = classify_huggingface_access_error(error)

    assert isinstance(classified, ModelAccessDeniedError)
    assert classified.repo_id == "owner/model"


def test_bare_500_http_error_is_not_classified_as_access_denied() -> None:
    """A server-side 5xx is not a credential/consent problem; leave it alone."""

    from huggingface_hub.errors import HfHubHTTPError

    error = HfHubHTTPError(
        "500 Server Error",
        response=_response(
            500, "https://huggingface.co/owner/model/resolve/main/file.bin"
        ),
    )

    assert classify_huggingface_access_error(error) is None


def test_unrelated_exception_is_not_classified() -> None:
    assert (
        classify_huggingface_access_error(ValueError("nothing to do with the Hub"))
        is None
    )
