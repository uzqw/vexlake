//! FFI exports for Go integration
//!
//! This module provides C-compatible FFI functions for the Go layer.
//! Uses Arrow C Data Interface for zero-copy data exchange.

use once_cell::sync::Lazy;
use std::ffi::{c_char, c_int, CString};
use std::panic::catch_unwind;
use std::sync::Mutex;

use crate::index::hnsw::{HnswConfig, HnswIndex};

static ENGINE: Lazy<Mutex<Option<HnswIndex>>> = Lazy::new(|| Mutex::new(None));

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
pub extern "C" fn vexlake_init(dim: c_int) -> c_int {
    catch_unwind(|| {
        let mut engine = ENGINE.lock().unwrap();
        let config = HnswConfig {
            dimension: dim as usize,
            ..Default::default()
        };
        *engine = Some(HnswIndex::new(config));
        0
    })
    .unwrap_or(-1)
}

/// Shutdown the VexLake engine
#[no_mangle]
pub extern "C" fn vexlake_shutdown() {
    let mut engine = ENGINE.lock().unwrap();
    *engine = None;
}

/// Insert a vector into the index
/// Returns 0 on success, negative on error
///
/// # Safety
/// The caller must ensure that `vec_ptr` points to a valid array of at least `len` f32 values.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vexlake_insert(id: u64, vec_ptr: *const f32, len: c_int) -> c_int {
    catch_unwind(|| {
        let mut engine_lock = ENGINE.lock().unwrap();
        if let Some(engine) = engine_lock.as_mut() {
            let vec = unsafe { std::slice::from_raw_parts(vec_ptr, len as usize) }.to_vec();
            if engine.insert(id, vec).is_ok() {
                return 0;
            }
        }
        -1
    })
    .unwrap_or(-1)
}

/// Search for the top K most similar vectors
/// Returns a JSON string of results (caller must free via vexlake_free_string)
///
/// # Safety
/// The caller must ensure that `query_ptr` points to a valid array of at least `len` f32 values.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vexlake_search(
    query_ptr: *const f32,
    len: c_int,
    k: c_int,
    ef: c_int,
) -> *mut c_char {
    let result = catch_unwind(|| {
        let engine_lock = ENGINE.lock().unwrap();
        if let Some(engine) = engine_lock.as_ref() {
            let query = unsafe { std::slice::from_raw_parts(query_ptr, len as usize) };
            if let Ok(results) = engine.search(query, k as usize, ef as usize) {
                if let Ok(json) = serde_json::to_string(&results) {
                    return CString::new(json).unwrap().into_raw();
                }
            }
        }
        std::ptr::null_mut()
    });

    match result {
        Ok(ptr) => ptr,
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a string allocated by Rust
///
/// # Safety
/// The caller must ensure that `ptr` was allocated by a Rust function in this library (e.g., `vexlake_search`).
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vexlake_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

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
        assert_eq!(vexlake_init(128), 0);
        vexlake_shutdown();
    }
}
