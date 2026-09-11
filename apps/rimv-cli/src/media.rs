use audio_core::{AudioFormat, AudioFrame, AudioSourceKind};
use speech_transcription::Preprocessor;
use std::{fs::File, path::Path, time::Duration};
use symphonia::core::{
    audio::sample::Sample,
    codecs::audio::AudioDecoderOptions,
    errors::Error as SymphoniaError,
    formats::{FormatOptions, TrackType, probe::Hint},
    io::MediaSourceStream,
    meta::MetadataOptions,
};

use crate::Result;

pub(crate) struct DecodedAudio {
    pub samples: Vec<f32>,
    pub duration_ms: u64,
}

/// Decode a supported local media file and normalize it through the same
/// mono/16 kHz preprocessor used by runtime transcription.
pub(crate) fn decode_mono_16k(path: &Path) -> Result<DecodedAudio> {
    let file = File::open(path)?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        hint.with_extension(extension);
    }
    let mut format = symphonia::default::get_probe().probe(
        &hint,
        stream,
        FormatOptions::default(),
        MetadataOptions::default(),
    )?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or("media file contains no audio track")?;
    let codec = track
        .codec_params
        .as_ref()
        .ok_or("audio track has no codec parameters")?
        .audio()
        .ok_or("selected track is not audio")?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(codec, &AudioDecoderOptions::default())?;
    let track_id = track.id;
    let mut preprocessor: Option<(u32, u16, Preprocessor)> = None;
    let mut samples = Vec::new();
    let mut input_frames = 0_u64;

    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(SymphoniaError::ResetRequired) => {
                return Err("media changed audio tracks while decoding".into());
            }
            Err(error) => return Err(error.into()),
        };
        if packet.track_id != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(_)) | Err(SymphoniaError::IoError(_)) => continue,
            Err(error) => return Err(error.into()),
        };
        let spec = decoded.spec();
        let rate = spec.rate();
        let channels = u16::try_from(spec.channels().count())
            .map_err(|_| "decoded channel count is too large")?;
        if preprocessor.is_none() {
            preprocessor = Some((rate, channels, Preprocessor::new(rate)?));
        }
        let (expected_rate, expected_channels, processor) =
            preprocessor.as_mut().expect("initialized above");
        if rate != *expected_rate || channels != *expected_channels {
            return Err("audio format changed while decoding".into());
        }
        let mut interleaved = vec![f32::MID; decoded.samples_interleaved()];
        decoded.copy_to_slice_interleaved(&mut interleaved);
        let frame_count = interleaved.len() / channels as usize;
        let timestamp = Duration::from_millis(input_frames.saturating_mul(1000) / rate as u64);
        input_frames = input_frames.saturating_add(frame_count as u64);
        let frame = AudioFrame::new(
            AudioSourceKind::Microphone,
            timestamp,
            AudioFormat::new(rate, channels)?,
            interleaved,
        )?;
        for block in processor.push(&frame)? {
            samples.extend(block.samples);
        }
    }

    let (rate, _, _) = preprocessor.ok_or("media file contained no decodable audio")?;
    if samples.is_empty() {
        return Err("media file did not contain enough decodable audio".into());
    }
    Ok(DecodedAudio {
        duration_ms: input_frames.saturating_mul(1000) / rate as u64,
        samples,
    })
}
