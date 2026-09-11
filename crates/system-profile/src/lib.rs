//! Rust-owned hardware facts used by rimv clients and recommendation services.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatingSystem {
    MacOS,
    Windows,
    Linux,
    Other,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CpuArchitecture {
    AppleSilicon,
    X86_64,
    Arm64,
    Other,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemProfile {
    pub os: OperatingSystem,
    pub os_version: String,
    pub architecture: CpuArchitecture,
    pub physical_memory_bytes: u64,
    pub logical_cpu_count: usize,
    pub apple_silicon: bool,
    pub metal_available: bool,
}
impl SystemProfile {
    pub fn detect() -> Self {
        let os = match std::env::consts::OS {
            "macos" => OperatingSystem::MacOS,
            "windows" => OperatingSystem::Windows,
            "linux" => OperatingSystem::Linux,
            _ => OperatingSystem::Other,
        };
        let arch = match std::env::consts::ARCH {
            "aarch64" if cfg!(target_os = "macos") => CpuArchitecture::AppleSilicon,
            "aarch64" => CpuArchitecture::Arm64,
            "x86_64" => CpuArchitecture::X86_64,
            _ => CpuArchitecture::Other,
        };
        let memory = memory_bytes();
        Self {
            os,
            os_version: std::env::var("RIMV_OS_VERSION").unwrap_or_default(),
            architecture: arch,
            physical_memory_bytes: memory,
            logical_cpu_count: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
            apple_silicon: arch == CpuArchitecture::AppleSilicon,
            metal_available: arch == CpuArchitecture::AppleSilicon,
        }
    }
}
#[cfg(target_os = "macos")]
fn memory_bytes() -> u64 {
    std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}
#[cfg(not(target_os = "macos"))]
fn memory_bytes() -> u64 {
    0
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_coherent_architecture() {
        let p = SystemProfile::detect();
        assert!(p.logical_cpu_count > 0);
        assert!(!p.apple_silicon || p.metal_available);
    }
}
