use engine_protocol::EngineCommand;

pub enum Input {
    Engine(EngineCommand),
    Quit,
    Help,
}
pub fn parse(line: &str) -> Result<Input, &'static str> {
    let words: Vec<_> = line.split_whitespace().collect();
    match words.as_slice() {
        ["start"] => Ok(Input::Engine(EngineCommand::StartCapture)),
        ["stop"] => Ok(Input::Engine(EngineCommand::StopCapture)),
        ["state"] => Ok(Input::Engine(EngineCommand::GetState)),
        ["quit" | "exit"] => Ok(Input::Quit),
        [] | ["help"] => Ok(Input::Help),
        [
            source @ ("mic" | "system" | "transcription"),
            value @ ("on" | "off"),
        ] => {
            let enabled = *value == "on";
            let command = match *source {
                "mic" => EngineCommand::SetMicrophoneEnabled { enabled },
                "system" => EngineCommand::SetSystemAudioEnabled { enabled },
                _ => EngineCommand::SetTranscriptionEnabled { enabled },
            };
            Ok(Input::Engine(command))
        }
        _ => Err(
            "use start, stop, mic on/off, system on/off, transcription on/off, state, help, quit",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_commands_instead_of_toggling_accidentally() {
        assert!(parse("mic yes").is_err());
        assert!(parse("start now").is_err());
        assert!(matches!(
            parse("mic off"),
            Ok(Input::Engine(EngineCommand::SetMicrophoneEnabled {
                enabled: false
            }))
        ));
    }
}
