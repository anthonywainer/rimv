use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=native/menu.m");
    println!("cargo:rerun-if-changed=Info.plist");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        _ => panic!("unsupported macOS architecture"),
    };
    let object = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("menu.o");
    let status = Command::new("xcrun")
        .args([
            "clang",
            "-arch",
            arch,
            "-mmacosx-version-min=13.0",
            "-fobjc-arc",
            "-fblocks",
            "-O2",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-c",
            "native/menu.m",
            "-o",
        ])
        .arg(&object)
        .status()
        .expect("Xcode command line tools are required for the menu-bar app");
    assert!(status.success(), "native menu compilation failed");
    println!("cargo:rustc-link-arg={}", object.display());
    println!("cargo:rustc-link-lib=framework=AppKit");
    println!("cargo:rustc-link-lib=framework=Foundation");
}
