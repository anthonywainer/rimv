# Speech integration fixtures

`../../resources/audio/audio1-16k-mono.wav` is derived from the checked-in
`resources/audio/audio1.mp3` speech sample. It is 16 kHz, mono, signed 16-bit
PCM, and 34.992 seconds long. The adjacent `audio1.txt` file is the reference
transcript. Generate it from the source with:

```sh
ffmpeg -i resources/audio/audio1.mp3 -ac 1 -ar 16000 -c:a pcm_s16le -map_metadata -1 resources/audio/audio1-16k-mono.wav
```

`asr_audio_fixture.rs` runs the real Parakeet and Silero implementation and
checks stable phrases from the reference. It is ignored by default because the
models are large; explicitly run it on a machine with the assets installed:

```sh
RIMV_PARAKEET_MODEL_DIR=/path/to/parakeet \
RIMV_SILERO_VAD_MODEL=/path/to/silero_vad.onnx \
cargo test -p speech-transcription --features parakeet,silero-vad --test asr_audio_fixture -- --ignored
```

`crates/engine-runtime/tests/apple_native_e2e.rs` streams the same checked-in
PCM fixture through RimV's capture adapter and the real Apple Speech provider,
then checks live partial replacement, final state, provider identity, and the
same reference phrase checks while reporting normalized WER/CER. Run it on a
macOS machine with on-device English (US) Speech support and permission:

```sh
cargo test -p engine-runtime --features apple-speech --test apple_native_e2e -- --ignored --nocapture
```
