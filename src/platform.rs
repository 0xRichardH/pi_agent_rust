//! Shared platform-identity utilities.
//!
//! Provides OS name, architecture, and client identity strings used by
//! provider request builders. Centralises the mapping from Rust's
//! `std::env::consts` values to the provider-expected naming conventions
//! (e.g. `darwin` instead of `macos`, `arm64` instead of `aarch64`).

/// Crate version baked in at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// OS
// ---------------------------------------------------------------------------

/// Return the OS name in the format most providers expect.
///
/// Maps Rust's `std::env::consts::OS`:
///   - `"macos"` → `"darwin"`
///   - everything else passed through (`"linux"`, `"windows"`, …)
#[inline]
pub fn os_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    }
}

// ---------------------------------------------------------------------------
// Architecture
// ---------------------------------------------------------------------------

/// Return the architecture name in the format most providers expect.
///
/// Maps Rust's `std::env::consts::ARCH`:
///   - `"aarch64"` → `"arm64"`
///   - `"x86_64"`  → `"amd64"`
///   - everything else passed through
#[inline]
pub fn arch_name() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        other => other,
    }
}

// ---------------------------------------------------------------------------
// Composite helpers
// ---------------------------------------------------------------------------

/// `"{os}/{arch}"` — e.g. `"linux/amd64"`, `"darwin/arm64"`.
pub fn platform_tag() -> String {
    format!("{}/{}", os_name(), arch_name())
}

/// Canonical Pi User-Agent: `"pi_agent_rust/{version}"`.
pub fn pi_user_agent() -> String {
    format!("pi_agent_rust/{VERSION}")
}

/// Canonical Pi User-Agent with an additional component:
/// `"pi_agent_rust/{version} {extra}"`.
pub fn pi_user_agent_with(extra: &str) -> String {
    format!("pi_agent_rust/{VERSION} {extra}")
}

use std::time::Duration;

const WINDOWS_IO_RETRY_LIMIT: usize = 10;

pub fn is_windows_transient_io_error_kind(kind: std::io::ErrorKind) -> bool {
    matches!(
        kind,
        std::io::ErrorKind::PermissionDenied
            | std::io::ErrorKind::AlreadyExists
            | std::io::ErrorKind::WouldBlock
    )
}

pub fn retry_io_with_backoff<T, F>(mut op: F) -> std::io::Result<T>
where
    F: FnMut() -> std::io::Result<T>,
{
    let mut attempts = 0;
    loop {
        match op() {
            Ok(value) => return Ok(value),
            Err(err) => {
                if !is_windows_transient_io_error_kind(err.kind()) {
                    return Err(err);
                }

                attempts += 1;
                if attempts > WINDOWS_IO_RETRY_LIMIT {
                    return Err(err);
                }

                std::thread::sleep(Duration::from_millis((5 * attempts) as u64));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn os_name_not_empty() {
        assert!(!os_name().is_empty());
    }

    #[test]
    fn arch_name_not_empty() {
        assert!(!arch_name().is_empty());
    }

    #[test]
    fn platform_tag_has_slash() {
        let tag = platform_tag();
        assert!(tag.contains('/'), "expected OS/ARCH, got: {tag}");
    }

    #[test]
    fn pi_user_agent_contains_version() {
        let ua = pi_user_agent();
        assert!(ua.starts_with("pi_agent_rust/"), "ua: {ua}");
        assert!(ua.contains(VERSION), "ua should contain version");
    }

    #[test]
    fn pi_user_agent_with_appends() {
        let ua = pi_user_agent_with("Antigravity/1.2.3");
        assert!(ua.starts_with("pi_agent_rust/"));
        assert!(ua.ends_with("Antigravity/1.2.3"));
    }

    #[test]
    fn retry_io_with_backoff_retries_permission_denied_then_succeeds() {
        let attempts = AtomicUsize::new(0);

        let value = retry_io_with_backoff(|| {
            let current = attempts.fetch_add(1, Ordering::SeqCst);
            if current < 2 {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "locked"));
            }

            Ok("ok")
        })
        .expect("retry should eventually succeed");

        assert_eq!(value, "ok");
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn retry_io_with_backoff_retries_would_block_then_succeeds() {
        let attempts = AtomicUsize::new(0);

        let value = retry_io_with_backoff(|| {
            let current = attempts.fetch_add(1, Ordering::SeqCst);
            if current < 1 {
                return Err(io::Error::new(io::ErrorKind::WouldBlock, "busy"));
            }

            Ok(7usize)
        })
        .expect("retry should eventually succeed");

        assert_eq!(value, 7);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn retry_io_with_backoff_does_not_retry_non_transient_errors() {
        let attempts = AtomicUsize::new(0);

        let err = retry_io_with_backoff::<(), _>(|| {
            attempts.fetch_add(1, Ordering::SeqCst);
            Err(io::Error::new(io::ErrorKind::NotFound, "missing"))
        })
        .expect_err("non-transient errors should not be retried");

        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_os_name() {
        assert_eq!(os_name(), "linux");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_maps_to_darwin() {
        assert_eq!(os_name(), "darwin");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn x86_64_maps_to_amd64() {
        assert_eq!(arch_name(), "amd64");
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn aarch64_maps_to_arm64() {
        assert_eq!(arch_name(), "arm64");
    }
}

// ---------------------------------------------------------------------------
// File System Ext
// ---------------------------------------------------------------------------

use std::path::Path;
use tempfile::NamedTempFile;

/// Extension trait for `NamedTempFile` to handle OS-specific persist issues.
pub trait NamedTempFileExt {
    /// Attempt to persist the file. On Windows, retries on "Access is denied"
    /// (os error 5 / PermissionDenied) and "AlreadyExists", which typically
    /// happen due to transient mandatory file locking (e.g. Antivirus).
    fn persist_with_retry<P: AsRef<Path>>(
        self,
        path: P,
    ) -> Result<std::fs::File, tempfile::PersistError>;
}

/// Extension trait for `TempPath` to handle OS-specific persist issues.
pub trait TempPathExt {
    fn persist_with_retry<P: AsRef<Path>>(self, path: P) -> Result<(), tempfile::PathPersistError>;
}

impl NamedTempFileExt for NamedTempFile {
    #[cfg(windows)]
    fn persist_with_retry<P: AsRef<Path>>(
        self,
        path: P,
    ) -> Result<std::fs::File, tempfile::PersistError> {
        let mut attempts = 0;
        let mut tmp = self;
        loop {
            match tmp.persist(path.as_ref()) {
                Ok(file) => return Ok(file),
                Err(e) => {
                    tmp = e.file;
                    let err = e.error;
                    if is_windows_transient_io_error_kind(err.kind()) {
                        attempts += 1;
                        if attempts > WINDOWS_IO_RETRY_LIMIT {
                            return Err(tempfile::PersistError {
                                file: tmp,
                                error: err,
                            });
                        }
                        std::thread::sleep(std::time::Duration::from_millis(5 * attempts));
                        continue;
                    }
                    return Err(tempfile::PersistError {
                        file: tmp,
                        error: err,
                    });
                }
            }
        }
    }

    #[cfg(not(windows))]
    fn persist_with_retry<P: AsRef<Path>>(
        self,
        path: P,
    ) -> Result<std::fs::File, tempfile::PersistError> {
        self.persist(path)
    }
}

impl TempPathExt for tempfile::TempPath {
    #[cfg(windows)]
    fn persist_with_retry<P: AsRef<Path>>(self, path: P) -> Result<(), tempfile::PathPersistError> {
        let mut attempts = 0;
        let mut tmp = self;
        loop {
            match tmp.persist(path.as_ref()) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tmp = e.path;
                    let err = e.error;
                    if is_windows_transient_io_error_kind(err.kind()) {
                        attempts += 1;
                        if attempts > WINDOWS_IO_RETRY_LIMIT {
                            return Err(tempfile::PathPersistError {
                                path: tmp,
                                error: err,
                            });
                        }
                        std::thread::sleep(std::time::Duration::from_millis(5 * attempts));
                        continue;
                    }
                    return Err(tempfile::PathPersistError {
                        path: tmp,
                        error: err,
                    });
                }
            }
        }
    }

    #[cfg(not(windows))]
    fn persist_with_retry<P: AsRef<Path>>(self, path: P) -> Result<(), tempfile::PathPersistError> {
        self.persist(path)
    }
}
