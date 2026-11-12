//! Process-level timing without extra dependencies: real process start (Windows
//! `GetProcessTimes`) and process CPU time, which is what S7 needs.
//!
//! All of this is best-effort: on a non-Windows host the numbers are simply absent
//! and the report says so rather than inventing a value.

use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 100 ns ticks between 1601-01-01 (FILETIME epoch) and 1970-01-01 (UNIX epoch).
const UNIX_EPOCH_FILETIME_TICKS: u64 = 116_444_736_000_000_000;
const TICKS_PER_SEC: f64 = 10_000_000.0;

#[cfg(windows)]
mod win {
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct FileTime {
        pub low: u32,
        pub high: u32,
    }

    impl FileTime {
        pub fn ticks(self) -> u64 {
            ((self.high as u64) << 32) | self.low as u64
        }
    }

    // kernel32 is linked by the Rust runtime on Windows, so declaring the two
    // functions we need keeps the spike free of a windows-sys dependency.
    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn GetCurrentProcess() -> isize;
        pub fn GetProcessTimes(
            h_process: isize,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }

    pub fn times() -> Option<(u64, u64)> {
        let mut creation = FileTime::default();
        let mut exit = FileTime::default();
        let mut kernel = FileTime::default();
        let mut user = FileTime::default();
        let ok = unsafe {
            GetProcessTimes(
                GetCurrentProcess(),
                &mut creation,
                &mut exit,
                &mut kernel,
                &mut user,
            )
        };
        if ok == 0 {
            return None;
        }
        Some((creation.ticks(), kernel.ticks() + user.ticks()))
    }
}

fn process_start_ticks() -> Option<u64> {
    #[cfg(windows)]
    {
        win::times().map(|(creation, _)| creation)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Real OS process start time, if the platform exposes it.
pub fn process_start() -> Option<SystemTime> {
    static START: OnceLock<Option<SystemTime>> = OnceLock::new();
    *START.get_or_init(|| {
        let ticks = process_start_ticks()?;
        let unix_ticks = ticks.checked_sub(UNIX_EPOCH_FILETIME_TICKS)?;
        Some(UNIX_EPOCH + Duration::from_nanos(unix_ticks * 100))
    })
}

/// Milliseconds since the operating system created this process.
pub fn since_process_start_ms() -> f64 {
    match process_start() {
        Some(start) => SystemTime::now()
            .duration_since(start)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(f64::NAN),
        None => f64::NAN,
    }
}

/// Accumulated kernel+user CPU time of this process.
pub fn cpu_time() -> Option<Duration> {
    #[cfg(windows)]
    {
        win::times().map(|(_, cpu)| Duration::from_nanos((cpu as f64 / TICKS_PER_SEC * 1e9) as u64))
    }
    #[cfg(not(windows))]
    {
        None
    }
}
