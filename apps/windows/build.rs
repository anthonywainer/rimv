fn main() {
    #[cfg(target_os = "windows")]
    tauri_build::build()
}
