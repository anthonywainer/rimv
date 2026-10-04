//! Process resource sampling shared by benchmark modes, with OS-specific reads
//! hidden behind one interface. CPU is reported as percent of total logical
//! machine capacity; memory is resident working set/RSS.
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Default)]
pub(crate) struct ResourceMetrics {
    pub average_cpu_percent: Option<f64>,
    pub peak_cpu_percent: Option<f64>,
    pub average_ram_bytes: Option<u64>,
    pub peak_ram_bytes: Option<u64>,
}

#[derive(Default)]
struct Samples {
    cpu_percent: Vec<f64>,
    ram_bytes: Vec<u64>,
}

pub(crate) struct ResourceMonitor {
    stop: Arc<AtomicBool>,
    samples: Arc<Mutex<Samples>>,
    thread: Option<JoinHandle<()>>,
}

impl ResourceMonitor {
    pub fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let samples = Arc::new(Mutex::new(Samples::default()));
        let worker_stop = stop.clone();
        let worker_samples = samples.clone();
        let thread = thread::Builder::new()
            .name("rimv-benchmark-resources".into())
            .spawn(move || {
                let logical_cpus =
                    thread::available_parallelism().map_or(1, |count| count.get()) as f64;
                let mut prior = process_sample();
                while !worker_stop.load(Ordering::Acquire) {
                    thread::sleep(Duration::from_millis(100));
                    let current = process_sample();
                    let Some((previous_cpu, previous_at, _)) = prior else {
                        prior = current;
                        continue;
                    };
                    let Some((cpu, at, rss)) = current else {
                        prior = None;
                        continue;
                    };
                    let wall_ns = at.duration_since(previous_at).as_nanos();
                    let cpu_ns = cpu.saturating_sub(previous_cpu);
                    if wall_ns > 0 {
                        let percent = cpu_ns as f64 * 100.0 / wall_ns as f64 / logical_cpus;
                        let mut samples = worker_samples.lock().unwrap_or_else(|p| p.into_inner());
                        samples.cpu_percent.push(percent.clamp(0.0, 100.0));
                        samples.ram_bytes.push(rss);
                    }
                    prior = Some((cpu, at, rss));
                }
            })
            .ok();
        Self {
            stop,
            samples,
            thread,
        }
    }

    pub fn finish(mut self) -> ResourceMetrics {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let samples = self.samples.lock().unwrap_or_else(|p| p.into_inner());
        ResourceMetrics {
            average_cpu_percent: average(&samples.cpu_percent),
            peak_cpu_percent: samples.cpu_percent.iter().copied().reduce(f64::max),
            average_ram_bytes: average_u64(&samples.ram_bytes),
            peak_ram_bytes: samples.ram_bytes.iter().copied().max(),
        }
    }
}

impl Drop for ResourceMonitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn average(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn average_u64(values: &[u64]) -> Option<u64> {
    (!values.is_empty())
        .then(|| values.iter().map(|v| u128::from(*v)).sum::<u128>() as u64 / values.len() as u64)
}

#[cfg(target_os = "macos")]
fn process_sample() -> Option<(u128, Instant, u64)> {
    use std::ffi::c_int;

    #[repr(C)]
    #[derive(Default)]
    struct TimeValue {
        seconds: c_int,
        microseconds: c_int,
    }
    #[repr(C)]
    #[derive(Default)]
    struct MachTaskBasicInfo {
        virtual_size: u64,
        resident_size: u64,
        resident_size_max: u64,
        user_time: TimeValue,
        system_time: TimeValue,
        policy: c_int,
        suspend_count: c_int,
    }
    type MachPort = u32;
    type MachInfoCount = u32;
    #[link(name = "System")]
    unsafe extern "C" {
        static mach_task_self_: MachPort;
        fn task_info(
            task: MachPort,
            flavor: c_int,
            info: *mut c_int,
            count: *mut MachInfoCount,
        ) -> c_int;
    }

    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: getrusage initializes the provided rusage structure on success.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: the successful getrusage call above initialized this value.
    let usage = unsafe { usage.assume_init() };
    let cpu_ns = timeval_ns(usage.ru_utime) + timeval_ns(usage.ru_stime);
    let mut info = MachTaskBasicInfo::default();
    let word_count = std::mem::size_of::<MachTaskBasicInfo>().div_ceil(std::mem::size_of::<c_int>())
        as MachInfoCount;
    let mut count = word_count;
    // SAFETY: info is writable storage for the full task info structure and
    // count reports the number of c_int words available.
    if unsafe {
        task_info(
            mach_task_self_,
            20,
            (&mut info as *mut MachTaskBasicInfo).cast(),
            &mut count,
        )
    } != 0
    {
        return None;
    }
    Some((cpu_ns, Instant::now(), info.resident_size))
}

#[cfg(target_os = "macos")]
fn timeval_ns(value: libc::timeval) -> u128 {
    value.tv_sec.max(0) as u128 * 1_000_000_000 + value.tv_usec.max(0) as u128 * 1_000
}

#[cfg(target_os = "linux")]
fn process_sample() -> Option<(u128, Instant, u64)> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let fields: Vec<&str> = stat.rsplit_once(") ")?.1.split_whitespace().collect();
    let user = fields.get(11)?.parse::<u128>().ok()?;
    let system = fields.get(12)?.parse::<u128>().ok()?;
    let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if ticks <= 0 {
        return None;
    }
    let cpu_ns = (user + system) * 1_000_000_000 / ticks as u128;
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let rss = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))?
        .split_whitespace()
        .next()?
        .parse::<u64>()
        .ok()?
        .saturating_mul(1024);
    Some((cpu_ns, Instant::now(), rss))
}

#[cfg(target_os = "windows")]
fn process_sample() -> Option<(u128, Instant, u64)> {
    use std::ffi::c_void;
    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    #[repr(C)]
    struct MemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn GetProcessTimes(
            process: *mut c_void,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut MemoryCounters,
            size: u32,
        ) -> i32;
    }
    let mut kernel = FileTime::default();
    let mut user = FileTime::default();
    let process = unsafe { GetCurrentProcess() };
    if unsafe {
        GetProcessTimes(
            process,
            &mut FileTime::default(),
            &mut FileTime::default(),
            &mut kernel,
            &mut user,
        )
    } == 0
    {
        return None;
    }
    let ticks = |time: FileTime| (u64::from(time.high) << 32) | u64::from(time.low);
    let cpu_ns = u128::from(ticks(kernel) + ticks(user)) * 100;
    let mut memory = MemoryCounters {
        cb: std::mem::size_of::<MemoryCounters>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };
    if unsafe { GetProcessMemoryInfo(process, &mut memory, memory.cb) } == 0 {
        return None;
    }
    Some((cpu_ns, Instant::now(), memory.working_set_size as u64))
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn process_sample() -> Option<(u128, Instant, u64)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_aggregates_are_defined_for_empty_and_measured_samples() {
        assert_eq!(average(&[]), None);
        assert_eq!(average(&[10.0, 30.0]), Some(20.0));
        assert_eq!(average_u64(&[100, 300]), Some(200));
    }

    #[test]
    fn monitor_samples_current_process_without_panicking() {
        let monitor = ResourceMonitor::start();
        thread::sleep(Duration::from_millis(230));
        let result = monitor.finish();
        assert!(result.peak_ram_bytes.is_some());
        assert!(result.average_cpu_percent.is_some());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_mach_task_layout_reports_a_plausible_resident_size() {
        let (_, _, resident_bytes) = process_sample().expect("read this process task info");
        assert!(resident_bytes > 0);
        assert!(
            resident_bytes < (1_u64 << 40),
            "implausible RSS: {resident_bytes}"
        );
    }
}
