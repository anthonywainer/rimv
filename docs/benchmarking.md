# Transcription benchmark methodology

The CLI benchmark writes `benchmark-results.json` and `benchmark-results.md` in
its output directory, plus raw per-case references, hypotheses, normalized
transcripts, metrics, and session event traces. It records the RimV version,
available environment profile, corpus path, timestamps, backend/model identity,
and normalization rule. Missing measurements are `null`/`n/a`; unsupported
platform engines must be reported as unsupported rather than replaced by a
different engine.

## Corpus

Install pinned preparation dependencies with `python3 -m pip install
datasets==3.6.0 huggingface-hub==0.30.2 numpy==2.2.4 soundfile==0.13.1
librosa==0.11.0`, then run:

```sh
python3 scripts/benchmark/prepare_fleurs.py ./benchmark-corpus/fleurs
python3 scripts/benchmark/prepare_extended.py ./benchmark-corpus/extended
```

FLEURS remains in its own output directory. It selects evenly spaced samples
from the test split for `en_us`, `es_419`, and `ru_ru`, pinned at revision
`d83087acf4dc369a38e49cc2da58296d2725e959`, licensed CC-BY-4.0. Cite the
dataset and authors ([FLEURS card](https://huggingface.co/datasets/google/fleurs)).
It represents read speech, not conversation or noise.

The extension script prepares (a) `edinburghcstr/ami`, `ihm/test`, revision
`46f28f2503e2ec48f8867a84eef356c70476beab`, CC-BY-4.0, for English meeting
conversation ([AMI card](https://huggingface.co/datasets/edinburghcstr/ami));
(b) deterministic noisy counterparts created by adding seeded Gaussian noise
at 10 dB SNR to those exact AMI samples; and (c) `facebook/voxpopuli`,
`en_accented/test`, revision `42f01879c780b4a2e90ec0b4f616c2ece526e4f1`,
CC0-1.0, for accented English, with speaker/accent fields retained
([VoxPopuli card](https://huggingface.co/datasets/facebook/voxpopuli)). The
manifest records source index, revision, reference hash and preparation
parameters. VoxPopuli's accented test split requires downloading its source
shards into the local Hugging Face cache; no data is checked in. AMI and
VoxPopuli are English-only here, so they do not extend the multilingual
comparison languages beyond FLEURS.

## Scoring

WER uses whitespace-delimited tokens; CER uses Unicode scalar values with
whitespace removed. Both apply the same deterministic normalization: Unicode
lowercase, retain Unicode alphanumeric code points, turn punctuation and
symbols into boundaries, and collapse whitespace. Therefore contractions split
at apostrophes, numbers remain digits (no number-to-word conversion), and
Unicode is not transliterated or canonically normalized. Raw references and
hypotheses are retained beside normalized copies. References must not be edited
to fit a model output.

## Measurements and limitations

Direct-file RTF is inference wall time divided by decoded audio duration;
model initialization is recorded separately. Realtime replay feeds paced PCM
through the shared `SpeechWorker`, VAD, partial/final decoder, and optional
Enhanced refiner. It is live-pipeline replay, not a hardware capture test.
Realtime-capture uses macOS ScreenCaptureKit and samples the RimV process while
audio plays. First-partial latency compares event arrival with the audio end
timestamp; finalization latency compares the first final event with the input
audio end. RTF is summed ASR inference time divided by summed input-audio time.
A deadline miss is an individual ASR inference whose duration exceeds the audio
duration it processed. Direct-file runs cannot supply partial/finalization
latencies. CPU/RAM fields that are not sampled remain unavailable. Do not
publish cross-OS Native comparisons as equivalent hardware results. Always
retain the full JSON and per-case artifacts and report unsupported engines,
failures, model versions, OS/hardware, and corpus details.

The repository's `resources/audio/audio1.mp3` is a CI smoke fixture, not an
accuracy corpus. The Windows Native engine remains hosted by the Windows app
bridge and is not currently exposed to the CLI replay runner. The README
contains the concise macOS-only headline table generated from
`benchmark-results/macos-full/benchmark-results.json`; it is not a
cross-platform result. Windows Native runtime coverage remains pending. The
checked-in summary contains aggregate metrics. Detailed case failures and
additional run metrics are recorded below. Complete raw per-case reports are
retained with the local run output and are not committed.

## Completed macOS benchmark

The 2026-10-04 replay used 600 samples per engine on Apple Silicon (macOS
version unavailable, 12 logical CPU cores, 24 GiB host RAM). The corpus used
100 read samples each from FLEURS English, Spanish, and Russian; 100 English
AMI meeting samples and deterministic 10 dB noisy versions of those samples;
and 100 accented English VoxPopuli samples. All 600 audio/reference pairs
passed manifest and hash checks. This was paced live-pipeline replay, not
hardware audio capture.

WER/CER are aggregated over successful cases. Other values below are averages
over successful cases unless identified as peaks. RAM and CPU are sampled for
the RimV process; Native Apple Speech also uses system services outside it.

| Engine | Successful | Failed | Failure details | Avg CPU | Peak CPU | Avg RAM | Peak RAM | Model load | Deadline misses |
|---|---:|---:|---|---:|---:|---:|---:|---:|---:|
| Native Apple | 244/600 | 356 | 249 no speech detected; 100 unsupported Russian locale; 7 no final transcript | 0.067% | 0.716% | 36 MB | 48 MB | 4 ms | 0 |
| Parakeet | 550/600 | 50 | 50 no final transcript | 8.426% | 46.339% | 1,478 MB | 1,744 MB | 584 ms | 0 |
| Whisper Small | 583/600 | 17 | 17 no final transcript | 24.502% | 33.901% | 832 MB | 1,029 MB | 128 ms | 657 |
| Enhanced — Experimental | 586/600 | 14 | 14 no final transcript | 11.291% | 57.485% | 1,981 MB | 2,524 MB | 693 ms | 0 |

First partial and finalization latency, WER/CER, RTF and peak process RAM are
shown in the README headline table. No engine dropped audio or work. Whisper
had 657 individual inference calls longer than the audio duration processed.
Native's failures include the unsupported Russian locale; its other no-speech
cases occurred in this corpus and should not be hidden when interpreting the
successful-case accuracy score.

Enhanced ran Parakeet for live partials and asynchronously refined finalized
segments with Whisper. Whisper refined 547 finals; Parakeet fallback remained
for 39. Of the refined finals, 154 improved WER, 225 worsened, and 168 kept the
same WER (70 changed text without changing WER). Its aggregate WER was 18.9%,
compared with 16.2% for Parakeet alone.

The full validated aggregate is checked in at
[`benchmark-results/macos-full/benchmark-results.json`](../benchmark-results/macos-full/benchmark-results.json).
Local per-case artifacts are excluded from the repository. The benchmark
results are specific to this machine and software configuration; Windows has
not been runtime-tested.

## Reproduction

Prepare data, install the required local model(s), and run a real-time pipeline
replay:

```sh
python3 scripts/benchmark/prepare_fleurs.py ./benchmark-corpus/fleurs --per-language 100
cargo run --release -p rimv -- benchmark --mode realtime-replay --corpus ./benchmark-corpus/fleurs --backend parakeet --model /path/to/parakeet --output ./benchmark-output

# Enhanced requires both installed models and never substitutes another engine.
cargo run --release -p rimv -- benchmark --mode realtime-replay --corpus ./benchmark-corpus/fleurs --backend enhanced --model /path/to/parakeet --refinement-model /path/to/whisper.bin --output ./benchmark-enhanced
```

Generate the README table from an artifact with:

```sh
python3 scripts/benchmark/update_readme.py ./benchmark-output/benchmark-results.json
```
