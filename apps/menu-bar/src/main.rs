#[cfg(target_os = "macos")]
mod macos;

fn main() -> std::process::ExitCode {
    #[cfg(target_os = "macos")]
    match macos::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rimv: {error}");
            std::process::ExitCode::FAILURE
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("The rimv menu-bar app requires macOS. Use engine-cli on this platform.");
        std::process::ExitCode::FAILURE
    }
}
