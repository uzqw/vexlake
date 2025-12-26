//! FFI exports for Go integration
//!
//! This module provides C-compatible FFI functions for the Go layer.
//! Uses Arrow C Data Interface for zero-copy data exchange.

use std::ffi::{c_char, c_int};
use std::panic::catch_unwind;

/// Health check - returns 1 if the library is functional
#[no_mangle]
pub extern "C" fn vexlake_health_check() -> c_int {
    1
}

/// Get the library version as a null-terminated string
/// Caller must NOT free this string (it's static)
#[no_mangle]
pub extern "C" fn vexlake_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

/// Initialize the VexLake engine
/// Returns 0 on success, negative on error
#[no_mangle]
pub extern "C" fn vexlake_init() -> c_int {
    catch_unwind(|| {
        // Initialize tracing, etc.
        0
    })
    .unwrap_or(-1)
}

/// Shutdown the VexLake engine
#[no_mangle]
pub extern "C" fn vexlake_shutdown() {
    // Cleanup resources
}

// TODO: Add FFI functions for:
// - vexlake_insert(key, vector, len) -> int
// - vexlake_search(query, len, k, results) -> int
// - vexlake_get(key, vector_out, len_out) -> int
// - vexlake_delete(key) -> int
// - Arrow C Data Interface exports

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_health_check() {
        assert_eq!(vexlake_health_check(), 1);
    }

    #[test]
    fn test_version() {
        let version_ptr = vexlake_version();
        assert!(!version_ptr.is_null());

        let version = unsafe { CStr::from_ptr(version_ptr) };
        assert!(!version.to_str().unwrap().is_empty());
    }

    #[test]
    fn test_init_shutdown() {
        assert_eq!(vexlake_init(), 0);
        vexlake_shutdown();
    }
}
