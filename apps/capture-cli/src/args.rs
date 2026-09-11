use clap::{Args as ClapArgs, Parser, Subcommand};
use std::path::PathBuf;
#[derive(Parser)]
#[command(
    name = "rimv-capture",
    version,
    about = "Capture microphone or true system audio to float32 WAV"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}
#[derive(Subcommand)]
pub enum Command {
    /// Enumerate input/output selectors and native supported configurations.
    Devices,
    Mic {
        #[command(flatten)]
        options: Options,
        #[arg(long, default_value = "microphone.wav")]
        output: PathBuf,
    },
    System {
        #[command(flatten)]
        options: Options,
        #[arg(long, default_value = "system.wav")]
        output: PathBuf,
    },
    /// Keep microphone and system streams separate; do not mix or synchronize.
    Both {
        #[command(flatten)]
        options: Options,
        #[arg(long, default_value = ".")]
        output_dir: PathBuf,
    },
}
#[derive(ClapArgs, Clone)]
pub struct Options {
    #[arg(long,default_value_t=10,value_parser=clap::value_parser!(u64).range(1..=86400))]
    pub seconds: u64,
    /// Device selector from devices. In both mode applies only to the microphone.
    #[arg(long)]
    pub device: Option<String>,
    #[arg(long)]
    pub sample_rate: Option<u32>,
    #[arg(long)]
    pub channels: Option<u16>,
    #[arg(long, default_value_t = 32)]
    pub buffer_capacity: usize,
}
impl Options {
    pub fn config(&self) -> audio_capture::CaptureConfig {
        audio_capture::CaptureConfig {
            device_id: self.device.clone(),
            preferred_sample_rate: self.sample_rate,
            preferred_channels: self.channels,
            buffer_capacity: self.buffer_capacity,
            ..Default::default()
        }
    }
}
