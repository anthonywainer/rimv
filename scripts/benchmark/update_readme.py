#!/usr/bin/env python3
"""Update the README benchmark table from a benchmark-results.json artifact."""

import argparse
import json
from pathlib import Path

START = "<!-- BENCHMARK_RESULTS_START -->"
END = "<!-- BENCHMARK_RESULTS_END -->"
ENGINES = ("Native", "Parakeet", "Whisper", "Enhanced")
DISPLAY_NAMES = {
    "Native": "Native Apple",
    "Parakeet": "Parakeet",
    "Whisper": "Whisper Small",
    "Enhanced": "Enhanced — Experimental",
}


def engine_name(case):
    name = case.get("metrics", {}).get("backend", "").casefold()
    if "enhanced" in name:
        return "Enhanced"
    if "parakeet" in name or "sherpa" in name:
        return "Parakeet"
    if "whisper" in name:
        return "Whisper"
    if "native" in name or "apple speech" in name or "windows speech" in name:
        return "Native"
    return None


def metric(cases, name):
    values = [case["metrics"].get(name) for case in cases]
    values = [value for value in values if isinstance(value, (int, float))]
    return f"{sum(values) / len(values):.3f}" if values else "—"


def weighted_wer(cases):
    scores = [case["metrics"].get("accuracy") for case in cases]
    scores = [score for score in scores if score]
    denominator = sum(score.get("reference_words", 0) for score in scores)
    errors = sum(score.get("substitutions", 0) + score.get("insertions", 0) + score.get("deletions", 0) for score in scores)
    if not scores:
        return "—"
    return errors / denominator if denominator else (0.0 if errors == 0 else 1.0)


def percent(value):
    return f"{value * 100:.1f}%" if isinstance(value, (int, float)) else "—"


def latency(value_or_cases, name=None):
    if name is None:
        value = value_or_cases
    else:
        values = [case["metrics"].get(name) for case in value_or_cases]
        values = [value for value in values if isinstance(value, (int, float))]
        value = sum(values) / len(values) if values else None
    return f"{value:,.0f} ms" if isinstance(value, (int, float)) else "—"


def format_peak_ram(value, native=False):
    if not isinstance(value, (int, float)):
        return "—"
    mebibytes = value / (1024 * 1024)
    if mebibytes >= 1000:
        return f"{mebibytes / 1000:.2f} GB"
    return f"{mebibytes:.0f} MB{'*' if native else ''}"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("results", type=Path)
    parser.add_argument("--readme", type=Path, default=Path("README.md"))
    args = parser.parse_args()
    data = json.loads(args.results.read_text(encoding="utf-8"))
    published = data.get("validation_status") in ("passed", "validated_with_case_failures")
    rows = []
    for engine in ENGINES:
        summary = data.get("engine_summaries", {}).get(engine)
        if published and summary:
            values = summary.get("metrics", {})
            wer = percent(values.get("wer"))
            cer = percent(values.get("cer"))
            first_partial = latency(values.get("first_partial_ms"))
            finalization = latency(values.get("finalization_latency_ms"))
            rtf = f"{values['rtf']:.3f}" if isinstance(values.get("rtf"), (int, float)) else "—"
            peak = format_peak_ram(values.get("peak_rss_bytes"), native=engine == "Native")
            rows.append(f"| {DISPLAY_NAMES[engine]} | {wer} | {cer} | {first_partial} | {finalization} | {rtf} | {peak} |")
            continue
        cases = [item for item in data.get("results", []) if published and item.get("status") == "ok" and engine_name(item) == engine]
        wer = percent(weighted_wer(cases)) if cases else "—"
        accuracy = [case["metrics"].get("accuracy", {}) for case in cases]
        reference_chars = sum(item.get("reference_characters", 0) for item in accuracy)
        character_errors = sum(item.get("character_substitutions", 0) + item.get("character_insertions", 0) + item.get("character_deletions", 0) for item in accuracy)
        cer = percent(character_errors / reference_chars) if reference_chars else "—"
        peak_bytes = max((case["metrics"].get("peak_rss_bytes") for case in cases if isinstance(case["metrics"].get("peak_rss_bytes"), (int, float))), default=None)
        rows.append(f"| {DISPLAY_NAMES[engine]} | {wer} | {cer} | {latency(cases, 'first_partial_ms')} | {latency(cases, 'finalization_latency_ms')} | {metric(cases, 'rtf')} | {format_peak_ram(peak_bytes, native=engine == 'Native')} |")
    block = "\n".join(["| Engine | WER ↓ | CER ↓ | First Partial ↓ | Finalization ↓ | RTF ↓ | Peak RAM |", "|---|---:|---:|---:|---:|---:|---:|", *rows])
    readme = args.readme.read_text(encoding="utf-8")
    before, marker, rest = readme.partition(START)
    if not marker:
        raise SystemExit(f"README missing marker: {START}")
    old, end_marker, after = rest.partition(END)
    if not end_marker:
        raise SystemExit(f"README missing marker: {END}")
    args.readme.write_text(before + START + "\n" + block + "\n" + END + after, encoding="utf-8")


if __name__ == "__main__":
    main()
