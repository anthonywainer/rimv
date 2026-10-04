#!/usr/bin/env python3
"""Prepare a deterministic small FLEURS corpus for RimV benchmark runs.

Requires: pip install datasets==3.6.0 numpy==2.2.4
FLEURS source revision is pinned below; the downloaded source is not copied into
the repository. Audio and original references are retained byte-for-byte where
possible; FLEURS samples are decoded and written as mono PCM16 WAV.
"""

import argparse
import hashlib
import json
import wave
from pathlib import Path

import numpy as np
from datasets import load_dataset

REVISION = "d83087acf4dc369a38e49cc2da58296d2725e959"
LANGUAGES = ("en_us", "es_419", "ru_ru")
ENGINE_LANGUAGE = {"en_us": "en", "es_419": "es", "ru_ru": "ru"}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("--per-language", type=int, default=100)
    args = parser.parse_args()
    if args.per_language < 1:
        parser.error("--per-language must be positive")
    args.output.mkdir(parents=True, exist_ok=True)
    manifest = []
    for language in LANGUAGES:
        rows = load_dataset(
            "google/fleurs",
            language,
            split="test",
            revision=REVISION,
            trust_remote_code=True,
        )
        # Evenly spaced indices provide deterministic coverage without RNG state.
        count = min(args.per_language, len(rows))
        indices = [(i * len(rows)) // count for i in range(count)]
        for ordinal, index in enumerate(indices):
            row = rows[index]
            name = f"fleurs-{language}-{ordinal:04d}"
            audio = row["audio"]
            samples = np.asarray(audio["array"], dtype=np.float32)
            if samples.ndim == 2:
                samples = samples.mean(axis=1)
            samples = np.clip(samples, -1.0, 1.0)
            pcm = (samples * 32767.0).round().astype("<i2")
            audio_path = args.output / f"{name}.wav"
            with wave.open(str(audio_path), "wb") as output:
                output.setnchannels(1)
                output.setsampwidth(2)
                output.setframerate(int(audio["sampling_rate"]))
                output.writeframes(pcm.tobytes())
            reference = str(row["transcription"])
            reference_path = args.output / f"{name}.txt"
            reference_path.write_text(reference, encoding="utf-8")
            metadata_path = args.output / f"{name}.json"
            metadata = {
                "language": ENGINE_LANGUAGE[language],
                "dataset_language": language,
                "dataset": "google/fleurs",
                "dataset_revision": REVISION,
                "split": "test",
                "source_index": index,
                "speaker_id": row.get("speaker_id"),
                "gender": row.get("gender"),
                "reference_sha256": hashlib.sha256(reference.encode()).hexdigest(),
                "audio_sha256": hashlib.sha256(pcm.tobytes()).hexdigest(),
                "sample_rate": int(audio["sampling_rate"]),
                "sample_count": int(len(pcm)),
            }
            metadata_path.write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            manifest.append({"case": name, "audio": audio_path.name, "reference": reference_path.name, "metadata": metadata_path.name})
    (args.output / "manifest.json").write_text(json.dumps({"dataset": "FLEURS", "revision": REVISION, "cases": manifest}, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
