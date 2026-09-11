use clap::{Parser, Subcommand};
use engine_protocol::AudioSource;
use speech_transcription::{SpeechConfig, SpeechToTextEngine, WhisperEngine};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "transcribe-cli",
    about = "Local Whisper transcription for rimv WAV files"
)]
struct Args {
    #[command(subcommand)]
    command: Command,
    #[arg(long, global = true)]
    model: PathBuf,
    #[arg(long, global = true)]
    language: Option<String>,
    #[arg(long, global = true)]
    threads: Option<usize>,
    #[arg(long, global = true)]
    cpu: bool,
}
#[derive(Subcommand)]
enum Command {
    File { input: PathBuf },
}

fn main() -> std::process::ExitCode {
    match run(Args::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("transcription failed: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let Command::File { input } = args.command;
    let mut reader = hound::WavReader::open(input)?;
    let spec = reader.spec();
    if spec.sample_rate != 16_000
        || spec.channels != 1
        || spec.sample_format != hound::SampleFormat::Float
    {
        return Err("file mode currently accepts mono 16 kHz float32 WAV; use rimv session WAV preprocessing through the runtime".into());
    }
    let samples: Vec<f32> = reader.samples::<f32>().collect::<Result<_, _>>()?;
    let mut config = SpeechConfig::default();
    config.model_path = Some(args.model);
    config.language = args.language;
    config.threads = args.threads.unwrap_or(config.threads);
    config.use_gpu = !args.cpu;
    let mut engine = WhisperEngine::load(config)?;
    for segment in engine.transcribe(&samples, 0)? {
        println!(
            "[{:>7}–{:>7} ms] {:?}: {}",
            segment.start_ms,
            segment.end_ms,
            AudioSource::Microphone,
            segment.text
        );
    }
    Ok(())
}
