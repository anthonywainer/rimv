fn main() {
    println!("cargo:rerun-if-changed=../speech-transcription/native/apple_speech.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    cc::Build::new()
        .file("../speech-transcription/native/apple_speech.m")
        .flag("-fobjc-arc")
        .flag("-fblocks")
        .compile("rimv_apple_speech");
    println!("cargo:rustc-link-lib=framework=Speech");
    println!("cargo:rustc-link-lib=framework=AVFoundation");
    println!("cargo:rustc-link-lib=framework=Foundation");
}
