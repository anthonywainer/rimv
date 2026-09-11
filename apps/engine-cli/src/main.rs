mod command;
use command::Input;
use engine_runtime::{EngineConfig, EngineEvent, EngineRuntime, SubscriptionError};
use std::io::{self, BufRead};

const HELP: &str = "Commands: start | stop | mic on/off | system on/off | transcription on/off | state | help | quit";

fn main() -> std::process::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(io::stderr)
        .init();
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = EngineConfig::default();
    if let Some(directory) = std::env::args_os().nth(1) {
        config.recordings_directory = directory.into();
    }
    let engine = EngineRuntime::new(config)?;
    let events = engine.subscribe()?;
    println!("rimv Engine v{}\n{HELP}", env!("CARGO_PKG_VERSION"));
    let listener = std::thread::Builder::new().name("engine-cli-events".into()).spawn(move || {
        loop {
            match events.recv() {
                Ok(EngineEvent::Snapshot { snapshot: s }) => println!(
                    "EVENT Snapshot #{}, {:?} | mic enabled={} active={} | system enabled={} active={} | transcription enabled={} available={} | elapsed={}ms | dropped mic={} system={} | session={}",
                    s.revision, s.status, s.microphone.enabled, s.microphone.active,
                    s.system_audio.enabled, s.system_audio.active, s.transcription.enabled, s.transcription.available,
                    s.elapsed_ms, s.dropped_microphone_blocks, s.dropped_system_blocks,
                    s.session.as_ref().map_or("-", |session| session.id.0.as_str())),
                Ok(EngineEvent::Error { error }) => eprintln!("EVENT Error: {error}"),
                Ok(EngineEvent::TranscriptPartial { segment }) => println!(
                    "EVENT Transcript partial [{:?} {}-{}ms]: {}",
                    segment.source, segment.start_ms, segment.end_ms, segment.text
                ),
                Ok(EngineEvent::TranscriptFinal { segment }) => println!(
                    "EVENT Transcript final [{:?} {}-{}ms]: {}",
                    segment.source, segment.start_ms, segment.end_ms, segment.text
                ),
                Ok(EngineEvent::TranscriptUpdate { update }) => println!(
                    "EVENT Transcript update [{} {:?} {}-{}ms final={}]: {}{}",
                    update.utterance_id,
                    update.source,
                    update.start_ms,
                    update.end_ms,
                    update.is_final,
                    update.stable_text,
                    update.unstable_text
                ),
                Ok(EngineEvent::TranscriptionError { error }) => {
                    eprintln!("EVENT Transcription error: {error}")
                }
                Err(SubscriptionError::Lagged { missed }) => eprintln!("Listener missed {missed} events; next snapshots contain full state"),
                Err(SubscriptionError::Closed) => break,
                Err(SubscriptionError::Timeout) => {},
            }
        }
    })?;
    let input_result = (|| -> io::Result<()> {
        for line in io::stdin().lock().lines() {
            match command::parse(&line?) {
                Ok(Input::Quit) => break,
                Ok(Input::Help) => println!("{HELP}"),
                Ok(Input::Engine(command)) => {
                    if let Err(error) = engine.send(command) {
                        eprintln!("COMMAND rejected: {error}");
                    }
                }
                Err(error) => eprintln!("{error}"),
            }
        }
        Ok(())
    })();
    // quit and EOF both finalize active WAVs and wake every listener.
    let shutdown_result = engine.shutdown();
    listener.join().map_err(|_| "event listener panicked")?;
    input_result?;
    shutdown_result?;
    Ok(())
}
