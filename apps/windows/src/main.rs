#[cfg(target_os = "windows")]
mod windows;

fn main() {
    #[cfg(target_os = "windows")]
    if let Err(error) = windows::run() {
        eprintln!("rimv: {error}");
        std::process::exit(1);
    }

    #[cfg(not(target_os = "windows"))]
    eprintln!("The RimV desktop shell is available on Windows.");
}
