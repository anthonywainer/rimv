use audio_core::{AudioFormat, AudioFrame};
use std::{
    fs::{File, OpenOptions},
    io::BufWriter,
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
/// Float WAV writer on the consumer thread. Files are never overwritten.
pub struct WavSink {
    path: PathBuf,
    file: Option<File>,
    writer: Option<hound::WavWriter<BufWriter<File>>>,
    format: Option<AudioFormat>,
    written: u64,
    target_seconds: u64,
    received: u64,
    peak: f32,
}
impl WavSink {
    pub fn new(path: &Path, seconds: u64) -> Result<Self> {
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok(Self {
            path: path.into(),
            file: Some(file),
            writer: None,
            format: None,
            written: 0,
            target_seconds: seconds,
            received: 0,
            peak: 0.0,
        })
    }
    pub fn write(&mut self, frame: &AudioFrame) -> Result<()> {
        let format = frame.format();
        if let Some(previous) = self.format {
            if previous != format {
                return Err("device format changed during WAV recording".into());
            }
        } else {
            let size =
                self.target_seconds * format.sample_rate() as u64 * format.channels() as u64 * 4;
            if size > u32::MAX as u64 - 128 {
                return Err(
                    "recording exceeds WAV's 4 GiB limit; choose a shorter duration".into(),
                );
            }
            let file = self.file.take().ok_or("WAV file already consumed")?;
            self.writer = Some(hound::WavWriter::new(
                BufWriter::new(file),
                hound::WavSpec {
                    channels: format.channels(),
                    sample_rate: format.sample_rate(),
                    bits_per_sample: 32,
                    sample_format: hound::SampleFormat::Float,
                },
            )?);
            self.format = Some(format);
        }
        self.received += frame.sample_frames() as u64;
        let target = self.target_seconds * format.sample_rate() as u64;
        let position =
            (frame.timestamp().as_secs_f64() * format.sample_rate() as f64).round() as u64;
        self.silence_until(position.min(target))?;
        let skip = self
            .written
            .saturating_sub(position)
            .min(frame.sample_frames() as u64) as usize;
        let count =
            (frame.sample_frames() - skip).min(target.saturating_sub(self.written) as usize);
        let writer = self.writer.as_mut().ok_or("WAV writer missing")?;
        for &sample in &frame.samples()
            [skip * format.channels() as usize..(skip + count) * format.channels() as usize]
        {
            self.peak = self.peak.max(sample.abs());
            writer.write_sample(sample)?;
        }
        self.written += count as u64;
        Ok(())
    }
    fn silence_until(&mut self, target: u64) -> Result<()> {
        let format = self.format.ok_or("WAV format missing")?;
        let writer = self.writer.as_mut().ok_or("WAV writer missing")?;
        for _ in self.written..target {
            for _ in 0..format.channels() {
                writer.write_sample(0.0f32)?;
            }
        }
        self.written = self.written.max(target);
        Ok(())
    }
    pub fn finish(mut self) -> Result<()> {
        let format = self.format.ok_or(
            "no audio received; check permissions, device and active playback (output is empty)",
        )?;
        self.silence_until(self.target_seconds * format.sample_rate() as u64)?;
        self.writer.take().ok_or("WAV writer missing")?.finalize()?;
        tracing::info!(path=%self.path.display(),seconds=self.target_seconds,channels=format.channels(),sample_rate=format.sample_rate(),received_sample_frames=self.received,peak=self.peak,"WAV finalized; timestamp gaps and trailing shortfall filled with silence");
        if self.peak == 0.0 {
            tracing::warn!(
                "all received samples were silent; verify permissions and play/speak audible content"
            );
        }
        if self.peak >= 1.0 {
            tracing::warn!(
                peak = self.peak,
                "audio reached or exceeded nominal full scale"
            );
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wav_roundtrip_preserves_stereo_and_gap_timing() -> Result<()> {
        let path = std::env::temp_dir().join(format!("rimv-wav-test-{}.wav", std::process::id()));
        let format = AudioFormat::new(10, 2)?;
        let mut sink = WavSink::new(&path, 1)?;
        sink.write(&AudioFrame::new(
            audio_core::AudioSourceKind::System,
            std::time::Duration::from_millis(200),
            format,
            vec![0.25, -0.25, 0.5, -0.5],
        )?)?;
        sink.finish()?;
        let mut reader = hound::WavReader::open(&path)?;
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.duration(), 10);
        let samples = reader
            .samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()?;
        assert_eq!(&samples[..8], &[0.0, 0.0, 0.0, 0.0, 0.25, -0.25, 0.5, -0.5]);
        std::fs::remove_file(path)?;
        Ok(())
    }
}
