#!/usr/bin/env python3
"""Prepare deterministic AMI conversation/noisy and VoxPopuli accent cases.

Install with: pip install datasets==3.6.0 numpy==2.2.4
Dataset bytes stay in the Hugging Face cache and generated corpus directory;
never commit them. FLEURS remains prepared by prepare_fleurs.py.
"""

import argparse
import hashlib
import json
import wave
from pathlib import Path

import numpy as np
from datasets import load_dataset
from huggingface_hub import hf_hub_download

AMI_REVISION = "46f28f2503e2ec48f8867a84eef356c70476beab"
VOXPOPULI_REVISION = "42f01879c780b4a2e90ec0b4f616c2ece526e4f1"
NOISE_SEED = 20261003


def load_test_split(repo, revision, pattern):
    shard_count = {
        ("edinburghcstr/ami", "ihm/test-*.parquet"): 4,
        ("facebook/voxpopuli", "en_accented/test-*.parquet"): 2,
    }.get((repo, pattern))
    if shard_count is None:
        raise ValueError(f"unsupported pinned test shard pattern: {repo} {pattern}")
    subset = pattern.split("/", maxsplit=1)[0]
    local_files = [
        hf_hub_download(
            repo,
            f"{subset}/test-{index:05d}-of-{shard_count:05d}.parquet",
            repo_type="dataset",
            revision=revision,
        )
        for index in range(shard_count)
    ]
    return load_dataset("parquet", data_files={"test": local_files}, split="test")


def write_case(root, name, samples, sample_rate, reference, metadata):
    samples = np.asarray(samples, dtype=np.float32).reshape(-1)
    samples = np.clip(samples, -1.0, 1.0)
    pcm = (samples * 32767.0).round().astype("<i2")
    audio_path = root / f"{name}.wav"
    with wave.open(str(audio_path), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(sample_rate)
        output.writeframes(pcm.tobytes())
    reference_path = root / f"{name}.txt"
    reference_path.write_text(reference, encoding="utf-8")
    metadata_path = root / f"{name}.json"
    metadata["reference_sha256"] = hashlib.sha256(reference.encode()).hexdigest()
    metadata["audio_sha256"] = hashlib.sha256(pcm.tobytes()).hexdigest()
    metadata["sample_rate"] = int(sample_rate)
    metadata["sample_count"] = int(len(pcm))
    metadata_path.write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return {"case": name, "audio": audio_path.name, "reference": reference_path.name, "metadata": metadata_path.name}


def add_ami(root, limit, noisy_snr_db):
    rows = load_test_split(
        "edinburghcstr/ami", AMI_REVISION, "ihm/test-*.parquet"
    )
    eligible = [i for i, row in enumerate(rows) if len(str(row["text"]).split()) >= 5]
    count = min(limit, len(eligible))
    indices = [eligible[(i * len(eligible)) // count] for i in range(count)]
    manifest = []
    for ordinal, index in enumerate(indices):
        row = rows[index]
        audio = row["audio"]
        samples = np.asarray(audio["array"], dtype=np.float32).reshape(-1)
        reference = str(row["text"])
        source = {
            "dataset": "edinburghcstr/ami",
            "revision": AMI_REVISION,
            "subset": "ihm/test",
            "source_index": index,
            "speaker_id": row.get("speaker_id"),
            "meeting_id": row.get("meeting_id"),
            "microphone_id": row.get("microphone_id"),
            "condition": "conversational_clean",
            "license": "CC-BY-4.0",
        }
        name = f"ami-test-{ordinal:04d}"
        manifest.append(write_case(root, name, samples, audio["sampling_rate"], reference, source.copy()))

        # Controlled additive Gaussian noise at fixed SNR. This preserves the
        # AMI reference and gives a deterministic noisy counterpart per utterance.
        signal_rms = float(np.sqrt(np.mean(np.square(samples, dtype=np.float64))))
        if signal_rms > 1e-8:
            seed_bytes = hashlib.sha256(f"{NOISE_SEED}:{name}".encode()).digest()[:8]
            seed = int.from_bytes(seed_bytes, "little")
            noise = np.random.Generator(np.random.PCG64(seed)).standard_normal(len(samples)).astype(np.float32)
            noise_rms = float(np.sqrt(np.mean(np.square(noise, dtype=np.float64))))
            scaled_noise = noise * (signal_rms / (10 ** (noisy_snr_db / 20) * max(noise_rms, 1e-8)))
            mixed = samples + scaled_noise
            peak = float(np.max(np.abs(mixed)))
            if peak > 1.0:
                mixed /= peak
            noisy_metadata = source.copy()
            noisy_metadata.update({"condition": "synthetic_awgn", "snr_db": noisy_snr_db, "noise_seed": seed})
            manifest.append(write_case(root, f"{name}-noise-{noisy_snr_db}db", mixed, audio["sampling_rate"], reference, noisy_metadata))
    return manifest


def add_accented_english(root, limit):
    rows = load_test_split(
        "facebook/voxpopuli", VOXPOPULI_REVISION, "en_accented/test-*.parquet"
    )
    eligible = [
        i
        for i, row in enumerate(rows)
        if len(str(row.get("raw_text") or row["normalized_text"]).split()) >= 5
    ]
    count = min(limit, len(eligible))
    indices = [eligible[(i * len(eligible)) // count] for i in range(count)]
    manifest = []
    for ordinal, index in enumerate(indices):
        row = rows[index]
        audio = row["audio"]
        reference = str(row.get("raw_text") or row["normalized_text"])
        metadata = {
            "language": "en",
            "dataset": "facebook/voxpopuli",
            "revision": VOXPOPULI_REVISION,
            "subset": "en_accented/test",
            "source_index": index,
            "speaker_id": row.get("speaker_id"),
            "gender": row.get("gender"),
            "accent": row.get("accent"),
            "condition": "accented_natural_speech",
            "license": "CC0-1.0",
        }
        manifest.append(write_case(root, f"voxpopuli-accented-{ordinal:04d}", audio["array"], audio["sampling_rate"], reference, metadata))
    return manifest


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("--ami-per-condition", type=int, default=100)
    parser.add_argument("--accented-count", type=int, default=100)
    parser.add_argument("--noise-snr-db", type=float, default=10.0)
    args = parser.parse_args()
    if args.ami_per_condition < 1 or args.accented_count < 1 or not np.isfinite(args.noise_snr_db):
        parser.error("sample counts must be positive and --noise-snr-db must be finite")
    args.output.mkdir(parents=True, exist_ok=True)
    cases = add_ami(args.output, args.ami_per_condition, args.noise_snr_db)
    cases.extend(add_accented_english(args.output, args.accented_count))
    manifest = {
        "schema_version": 1,
        "preparation_version": "1",
        "noise_recipe": {"type": "AWGN", "snr_db": args.noise_snr_db, "seed_base": NOISE_SEED, "generator": "NumPy PCG64", "numpy_version": np.__version__},
        "sources": [
            {"dataset": "edinburghcstr/ami", "revision": AMI_REVISION, "subset": "ihm/test", "license": "CC-BY-4.0"},
            {"dataset": "facebook/voxpopuli", "revision": VOXPOPULI_REVISION, "subset": "en_accented/test", "license": "CC0-1.0"},
        ],
        "cases": cases,
    }
    (args.output / "manifest-extended.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
