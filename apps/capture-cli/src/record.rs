use crate::{
    args::{Args, Command},
    wav::WavSink,
};
use audio_capture::{AudioCapture, FrameReceiver, MicrophoneCapture, SystemAudioCapture};
use std::time::{Duration, Instant};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
struct Session {
    capture: Box<dyn AudioCapture>,
    receiver: FrameReceiver,
    sink: WavSink,
}
pub fn run(args: Args) -> Result<()> {
    let mut sessions = Vec::new();
    let seconds = match args.command {
        Command::Devices => {
            for (label, devices) in [
                ("Inputs", audio_capture::list_microphones()?),
                ("Outputs", audio_capture::list_output_devices()?),
            ] {
                println!("{label}:");
                for device in devices {
                    println!(
                        "  {} {} ({})",
                        if device.is_default { "*" } else { " " },
                        device.name,
                        device.id
                    );
                    let configs = if label == "Inputs" {
                        audio_capture::supported_microphone_configs(&device.id)
                    } else {
                        audio_capture::supported_output_configs(&device.id)
                    };
                    match configs {
                        Ok(configs) => {
                            for c in configs {
                                println!(
                                    "      {} ch, {}..={} Hz, {}",
                                    c.channels,
                                    c.min_sample_rate,
                                    c.max_sample_rate,
                                    c.sample_format
                                );
                            }
                        }
                        Err(e) => println!("      configs unavailable: {e}"),
                    }
                }
            }
            return Ok(());
        }
        Command::Mic { options, output } => {
            let (capture, receiver) = MicrophoneCapture::new(options.config())?;
            sessions.push(Session {
                capture: Box::new(capture),
                receiver,
                sink: WavSink::new(&output, options.seconds)?,
            });
            options.seconds
        }
        Command::System { options, output } => {
            let (capture, receiver) = SystemAudioCapture::new(options.config())?;
            sessions.push(Session {
                capture: Box::new(capture),
                receiver,
                sink: WavSink::new(&output, options.seconds)?,
            });
            options.seconds
        }
        Command::Both {
            options,
            output_dir,
        } => {
            let mut config = options.config();
            config.device_id = None;
            let (system, system_receiver) = SystemAudioCapture::new(config)?;
            let (mic, mic_receiver) = MicrophoneCapture::new(options.config())?;
            sessions.push(Session {
                capture: Box::new(mic),
                receiver: mic_receiver,
                sink: WavSink::new(&output_dir.join("microphone.wav"), options.seconds)?,
            });
            sessions.push(Session {
                capture: Box::new(system),
                receiver: system_receiver,
                sink: WavSink::new(&output_dir.join("system.wav"), options.seconds)?,
            });
            options.seconds
        }
    };
    for session in &mut sessions {
        session.capture.start()?;
    }
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let result = (|| -> Result<()> {
        while Instant::now() < deadline {
            for session in &mut sessions {
                if let Some(frame) = session.receiver.recv_timeout(
                    Duration::from_millis(5)
                        .min(deadline.saturating_duration_since(Instant::now())),
                )? {
                    session.sink.write(&frame)?;
                }
                if !session.capture.is_running() {
                    return Err("capture stopped unexpectedly".into());
                }
            }
        }
        Ok(())
    })();
    let mut failure = result.err();
    for session in &mut sessions {
        if let Err(e) = session.capture.stop()
            && failure.is_none()
        {
            failure = Some(e.into());
        }
        tracing::info!(metrics=?session.capture.metrics(),"capture metrics");
    }
    for mut session in sessions {
        let drain = (|| -> Result<()> {
            while let Some(frame) = session.receiver.recv_timeout(Duration::ZERO)? {
                session.sink.write(&frame)?;
            }
            Ok(())
        })();
        if failure.is_none() {
            failure = drain.err();
        }
        let finish = session.sink.finish();
        if failure.is_none() {
            failure = finish.err();
        }
    }
    if let Some(e) = failure {
        Err(e)
    } else {
        Ok(())
    }
}
