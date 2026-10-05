//! Irreversible resource ceilings for a disposable worker process.
//!
//! Call only at worker startup, before accepting media or other untrusted work.
//! This does not create a filesystem/network/syscall sandbox. Virtual-address
//! growth is not a resident-memory limit; the platform's pre-existing mappings
//! and reserved arenas remain part of the measured startup baseline.
use serde::{Deserialize, Serialize};
use std::{fmt, io};
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[allow(unsafe_code)]
mod signals;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProcessLimits {
    /// Maximum newly mapped virtual bytes beyond the measured worker baseline.
    pub additional_address_bytes: u64,
    /// Total process CPU seconds (all calls), not a renewable per-call timeout.
    pub cpu_seconds: u32,
    pub open_files: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppliedLimits {
    pub startup_virtual_bytes: u64,
    pub address_ceiling_bytes: u64,
    pub cpu_seconds: u64,
    pub open_files: u64,
    pub core_file_bytes: u64,
    pub regular_file_bytes: u64,
}
#[derive(Debug)]
pub enum LimitError {
    Invalid(&'static str),
    Unsupported,
    Platform(io::Error),
    Verify(&'static str),
}
impl fmt::Display for LimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "worker resource limits: {self:?}")
    }
}
impl std::error::Error for LimitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Platform(error) => Some(error),
            _ => None,
        }
    }
}
impl From<io::Error> for LimitError {
    fn from(error: io::Error) -> Self {
        Self::Platform(error)
    }
}
impl ProcessLimits {
    pub fn validate(self) -> Result<(), LimitError> {
        if !(16 << 20..=64 << 30).contains(&self.additional_address_bytes) {
            return Err(LimitError::Invalid(
                "additional address budget must be 16 MiB through 64 GiB",
            ));
        }
        if !(1..=86400).contains(&self.cpu_seconds) {
            return Err(LimitError::Invalid(
                "CPU budget must be 1 through 86400 seconds",
            ));
        }
        if !(3..=4096).contains(&self.open_files) {
            return Err(LimitError::Invalid(
                "file descriptor budget must be 3 through 4096",
            ));
        }
        Ok(())
    }
}

/// Apply, then read back every ceiling. Never raise an inherited soft/hard limit.
/// A platform failure may follow partial installation: the caller MUST terminate
/// the disposable worker on any error, never continue computation unconstrained.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn apply_to_current_worker(limits: ProcessLimits) -> Result<AppliedLimits, LimitError> {
    use rustix::process::{Resource, Rlimit, getrlimit, setrlimit};
    limits.validate()?;
    let baseline = platform::virtual_size()?;
    let address = baseline
        .checked_add(limits.additional_address_bytes)
        .ok_or(LimitError::Invalid("address ceiling overflow"))?;
    let cap = |resource, requested: u64| {
        let old = getrlimit(resource);
        requested
            .min(old.current.unwrap_or(u64::MAX))
            .min(old.maximum.unwrap_or(u64::MAX))
    };
    let address = cap(Resource::As, address);
    let cpu = cap(Resource::Cpu, limits.cpu_seconds.into());
    let files = cap(Resource::Nofile, limits.open_files.into());
    if address <= baseline || cpu == 0 || files < 3 {
        return Err(LimitError::Invalid(
            "inherited limits do not permit worker startup",
        ));
    }
    // exec may preserve ignored/blocked signals from the host. Establish the
    // worker's own terminating policy before relying on CPU/file ceilings.
    signals::install()?;
    let entries = [
        (Resource::Core, 0, "core file"),
        (Resource::Fsize, 0, "regular file"),
        (Resource::Nofile, files, "file descriptors"),
        (Resource::As, address, "virtual address"),
        (Resource::Cpu, cpu, "CPU"),
    ];
    for (resource, value, label) in entries {
        let expected = Rlimit {
            current: Some(value),
            maximum: Some(value),
        };
        setrlimit(resource, expected).map_err(|e| LimitError::Platform(e.into()))?;
        if getrlimit(resource) != expected {
            return Err(LimitError::Verify(label));
        }
    }
    Ok(AppliedLimits {
        startup_virtual_bytes: baseline,
        address_ceiling_bytes: address,
        cpu_seconds: cpu,
        open_files: files,
        core_file_bytes: 0,
        regular_file_bytes: 0,
    })
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn apply_to_current_worker(limits: ProcessLimits) -> Result<AppliedLimits, LimitError> {
    limits.validate()?;
    Err(LimitError::Unsupported)
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod platform {
    use std::{
        io,
        mem::{MaybeUninit, size_of},
    };
    pub fn virtual_size() -> io::Result<u64> {
        let mut info = MaybeUninit::<libc::proc_taskinfo>::uninit();
        // SAFETY: owned output sized exactly for PROC_PIDTASKINFO. Only the
        // calling worker is queried, and no output is read before full success.
        let received = unsafe {
            libc::proc_pidinfo(
                std::process::id() as i32,
                libc::PROC_PIDTASKINFO,
                0,
                info.as_mut_ptr().cast(),
                size_of::<libc::proc_taskinfo>() as i32,
            )
        };
        if received <= 0 {
            return Err(io::Error::last_os_error());
        }
        if received != size_of::<libc::proc_taskinfo>() as i32 {
            return Err(io::Error::other("incomplete process memory record"));
        }
        let info = unsafe { info.assume_init() };
        if info.pti_virtual_size == 0 {
            return Err(io::Error::other("worker virtual size unavailable"));
        }
        Ok(info.pti_virtual_size)
    }
}
#[cfg(target_os = "linux")]
mod platform {
    use std::{
        fs::File,
        io::{self, Read},
    };
    pub fn virtual_size() -> io::Result<u64> {
        let mut text = String::new();
        File::open("/proc/self/statm")?
            .take(4096)
            .read_to_string(&mut text)?;
        let pages = text
            .split_whitespace()
            .next()
            .and_then(|n| n.parse::<u64>().ok())
            .filter(|n| *n > 0)
            .ok_or_else(|| io::Error::other("worker virtual size unavailable"))?;
        pages
            .checked_mul(rustix::param::page_size() as u64)
            .ok_or_else(|| io::Error::other("worker virtual size overflow"))
    }
}
