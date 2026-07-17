"""Machine-ear verify engine: does the child in this clip say this text?

One of the three placement-verification engines behind `merge-verify`
(with forced-alignment confirmation and pitch banding). Puts one
(audio clip, candidate text) question to a local audio-LLM and parses
the answer into a YES / NO / UNPARSEABLE verdict.

CALIBRATION-LOCKED: the default model (`nvidia/audio-flamingo-3-hf`),
CPU inference, greedy decoding, the prompt wording, and the 16 kHz
clip rate all match the pipeline that was human-calibrated in July
2026 (97.5% measured auto-trust precision when composed with the
pitch-band leg). Changing the model, prompt, or decoding invalidates
that calibration; recalibrate against blind listening verdicts before
shipping such a change.

SECURITY INVARIANT: models load WITHOUT ``trust_remote_code``, always.
This engine runs on machines holding contributor data; executing
third-party hub code there is never an engine-level decision. A model
family whose loader would need remote code is simply not loadable here.

Environment note: the calibrated flamingo family needs a transformers
release that ships ``AudioFlamingo3ForConditionalGeneration`` (the
calibration ran on transformers 5.x); the loader reports that
requirement precisely when the installed version is too old.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Any

SAMPLE_RATE = 16_000
CALIBRATED_MODEL_ID = "nvidia/audio-flamingo-3-hf"
MAX_ANSWER_TOKENS = 6


class EarVerdict(Enum):
    """The model's parsed answer for one (clip, text) question."""

    YES = "yes"
    NO = "no"
    UNPARSEABLE = "unparseable"


@dataclass(frozen=True, slots=True)
class ClipEarAnswer:
    """One probed clip: the parsed verdict plus the model's raw answer."""

    verdict: EarVerdict
    raw_answer: str


class MachineEarHandle:
    """Loaded audio-LLM pieces needed to probe clips.

    ``processor`` and ``model`` are transformers objects whose concrete
    classes vary by model family; the factory surface upstream is
    untyped, so the fields are deliberately loose and every use stays
    inside this module.
    """

    def __init__(self, processor: Any, model: Any, device: str) -> None:
        self.processor = processor
        self.model = model
        self.device = device


def ear_prompt(text: str) -> str:
    """The calibrated question wording for one candidate text."""
    return (
        "A child and adults are talking. Does the CHILD in this "
        f'clip say: "{text}"? Answer with exactly one '
        "word: YES or NO."
    )


def parse_verdict(answer: str) -> EarVerdict:
    """Map the model's raw answer onto a verdict; anything else is honest noise."""
    lowered = answer.strip().lower()
    if lowered.startswith("yes"):
        return EarVerdict.YES
    if lowered.startswith("no"):
        return EarVerdict.NO
    return EarVerdict.UNPARSEABLE


def load_machine_ear(
    model_id: str = CALIBRATED_MODEL_ID, device: str = "cpu"
) -> MachineEarHandle:
    """Load (processor, model) for a supported audio-LLM family.

    Family dispatch by model id, mirroring the calibration pipeline's
    loader minus its bake-off-only remote-code branch (see the module
    security invariant).
    """
    # Heavy imports kept local so module import stays cheap for callers
    # that only need the verdict types and prompt.
    import torch
    from transformers import AutoProcessor

    from batchalign.worker._progress import emit_download_event

    emit_download_event(
        stage="downloading_machine_ear_model",
        user_message=(
            f"Downloading machine-ear model {model_id} for placement "
            "verification (one-time; future runs will use the local cache)…"
        ),
    )
    dtype = torch.float16 if device == "mps" else torch.float32
    lowered = model_id.lower()
    # transformers' AutoProcessor factory is untyped upstream; narrow ignore.
    processor = AutoProcessor.from_pretrained(model_id)  # type: ignore[no-untyped-call]
    model: Any
    if "qwen2-audio" in lowered:
        from transformers import Qwen2AudioForConditionalGeneration

        model = Qwen2AudioForConditionalGeneration.from_pretrained(model_id, dtype=dtype)
    elif "audio-flamingo-3" in lowered:
        try:
            from transformers import AudioFlamingo3ForConditionalGeneration
        except ImportError as err:
            msg = (
                f"{model_id} needs a transformers release with "
                "AudioFlamingo3ForConditionalGeneration (the calibration ran "
                "on transformers 5.x); the installed version lacks it"
            )
            raise ValueError(msg) from err

        model = AudioFlamingo3ForConditionalGeneration.from_pretrained(model_id, dtype=dtype)
    else:
        msg = f"no loader for {model_id}; add a family branch (no trust_remote_code)"
        raise ValueError(msg)
    return MachineEarHandle(
        processor=processor, model=model.to(torch.device(device)), device=device
    )


def ask_clip(handle: MachineEarHandle, wav: Path, text: str) -> ClipEarAnswer:
    """Put one (clip, text) question to the loaded model; parse YES/NO.

    Reads the clip itself (the calibration pipeline hands the model both
    the file reference for the chat template and the decoded samples);
    a clip not at the calibrated 16 kHz raises rather than silently
    mis-hearing.
    """
    import soundfile
    import torch

    audio, rate = soundfile.read(str(wav), dtype="float32")
    if rate != SAMPLE_RATE:
        msg = f"{wav}: machine ear is calibrated at {SAMPLE_RATE} Hz; got {rate}"
        raise ValueError(msg)
    conversation = [
        {"role": "user", "content": [
            {"type": "audio", "audio_url": str(wav)},
            {"type": "text", "text": ear_prompt(text)},
        ]},
    ]
    prompt = handle.processor.apply_chat_template(
        conversation, add_generation_prompt=True, tokenize=False
    )
    inputs = handle.processor(
        text=prompt, audio=[audio], sampling_rate=SAMPLE_RATE, return_tensors="pt"
    ).to(handle.device)
    # Some processors (audio-flamingo-3) emit fp32 features while the
    # model runs fp16 on MPS; cast floating inputs to the model dtype.
    model_dtype = next(handle.model.parameters()).dtype
    for key, value in inputs.items():
        if hasattr(value, "is_floating_point") and value.is_floating_point():
            inputs[key] = value.to(model_dtype)
    with torch.inference_mode():
        generated = handle.model.generate(
            **inputs, max_new_tokens=MAX_ANSWER_TOKENS, do_sample=False
        )
    answer = handle.processor.batch_decode(
        generated[:, inputs["input_ids"].shape[1]:], skip_special_tokens=True
    )[0].strip()
    return ClipEarAnswer(verdict=parse_verdict(answer), raw_answer=answer)
