//! FFI (Foreign Function Interface) for CRA Core
//!
//! This module provides a C-compatible API that allows any language
//! to use CRA Core through standard C FFI bindings.
//!
//! ## Thread Safety
//!
//! The resolver is NOT thread-safe. Each thread should have its own
//! resolver instance, or synchronization should be handled by the caller.
//!
//! ## Memory Management
//!
//! - All strings returned by this API are heap-allocated and must be freed
//!   using `cra_free_string`.
//! - All opaque pointers must be freed using their respective `*_free` functions.
//! - Strings passed to this API must be valid UTF-8 and null-terminated.
//!
//! ## Error Handling
//!
//! - Functions return null pointers on error.
//! - Use `cra_get_last_error` to retrieve error messages.
//! - Error messages are thread-local.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::cell::RefCell;

use crate::atlas::AtlasManifest;
use crate::carp::{CARPRequest, Resolver};

// Thread-local storage for error messages
thread_local! {
    static LAST_ERROR: RefCell<Option<String>> = RefCell::new(None);
}

/// Set the last error message
fn set_error(msg: String) {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = Some(msg);
    });
}

/// Clear the last error
fn clear_error() {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = None;
    });
}

// ============================================================================
// String Helpers
// ============================================================================

/// Get the last error message.
///
/// Returns null if no error occurred.
/// The returned string must be freed with `cra_free_string`.
#[no_mangle]
pub extern "C" fn cra_get_last_error() -> *mut c_char {
    LAST_ERROR.with(|e| {
        match &*e.borrow() {
            Some(msg) => {
                CString::new(msg.as_str())
                    .map(|s| s.into_raw())
                    .unwrap_or(ptr::null_mut())
            }
            None => ptr::null_mut(),
        }
    })
}

/// Free a string returned by this API.
#[no_mangle]
pub extern "C" fn cra_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Convert a C string to a Rust string.
unsafe fn c_str_to_string(s: *const c_char) -> Option<String> {
    if s.is_null() {
        return None;
    }
    CStr::from_ptr(s).to_str().ok().map(|s| s.to_string())
}

/// Convert a Rust string to a C string.
fn string_to_c(s: &str) -> *mut c_char {
    CString::new(s)
        .map(|s| s.into_raw())
        .unwrap_or(ptr::null_mut())
}

// ============================================================================
// Resolver API
// ============================================================================

/// Opaque handle to a Resolver
pub struct CRAResolver {
    inner: Resolver,
}

/// Create a new CRA resolver.
///
/// Returns null on error.
/// The resolver must be freed with `cra_resolver_free`.
#[no_mangle]
pub extern "C" fn cra_resolver_new() -> *mut CRAResolver {
    clear_error();
    Box::into_raw(Box::new(CRAResolver {
        inner: Resolver::new(),
    }))
}

/// Free a resolver.
#[no_mangle]
pub extern "C" fn cra_resolver_free(resolver: *mut CRAResolver) {
    if !resolver.is_null() {
        unsafe {
            drop(Box::from_raw(resolver));
        }
    }
}

/// Load an atlas from a JSON string.
///
/// Returns the atlas ID on success, null on error.
/// The returned string must be freed with `cra_free_string`.
#[no_mangle]
pub extern "C" fn cra_resolver_load_atlas_json(
    resolver: *mut CRAResolver,
    json: *const c_char,
) -> *mut c_char {
    clear_error();

    let resolver = unsafe {
        match resolver.as_mut() {
            Some(r) => r,
            None => {
                set_error("Null resolver pointer".to_string());
                return ptr::null_mut();
            }
        }
    };

    let json_str = match unsafe { c_str_to_string(json) } {
        Some(s) => s,
        None => {
            set_error("Null or invalid JSON string".to_string());
            return ptr::null_mut();
        }
    };

    let manifest: AtlasManifest = match serde_json::from_str(&json_str) {
        Ok(m) => m,
        Err(e) => {
            set_error(format!("Failed to parse atlas JSON: {}", e));
            return ptr::null_mut();
        }
    };

    match resolver.inner.load_atlas(manifest) {
        Ok(id) => string_to_c(&id),
        Err(e) => {
            set_error(format!("Failed to load atlas: {}", e));
            ptr::null_mut()
        }
    }
}

/// Unload an atlas.
///
/// Returns 0 on success, -1 on error.
#[no_mangle]
pub extern "C" fn cra_resolver_unload_atlas(
    resolver: *mut CRAResolver,
    atlas_id: *const c_char,
) -> i32 {
    clear_error();

    let resolver = unsafe {
        match resolver.as_mut() {
            Some(r) => r,
            None => {
                set_error("Null resolver pointer".to_string());
                return -1;
            }
        }
    };

    let atlas_id_str = match unsafe { c_str_to_string(atlas_id) } {
        Some(s) => s,
        None => {
            set_error("Null or invalid atlas ID".to_string());
            return -1;
        }
    };

    match resolver.inner.unload_atlas(&atlas_id_str) {
        Ok(()) => 0,
        Err(e) => {
            set_error(format!("Failed to unload atlas: {}", e));
            -1
        }
    }
}

/// Create a new session.
///
/// Returns the session ID on success, null on error.
/// The returned string must be freed with `cra_free_string`.
#[no_mangle]
pub extern "C" fn cra_resolver_create_session(
    resolver: *mut CRAResolver,
    agent_id: *const c_char,
    goal: *const c_char,
) -> *mut c_char {
    clear_error();

    let resolver = unsafe {
        match resolver.as_mut() {
            Some(r) => r,
            None => {
                set_error("Null resolver pointer".to_string());
                return ptr::null_mut();
            }
        }
    };

    let agent_id_str = match unsafe { c_str_to_string(agent_id) } {
        Some(s) => s,
        None => {
            set_error("Null or invalid agent ID".to_string());
            return ptr::null_mut();
        }
    };

    let goal_str = match unsafe { c_str_to_string(goal) } {
        Some(s) => s,
        None => {
            set_error("Null or invalid goal".to_string());
            return ptr::null_mut();
        }
    };

    match resolver.inner.create_session(&agent_id_str, &goal_str) {
        Ok(id) => string_to_c(&id),
        Err(e) => {
            set_error(format!("Failed to create session: {}", e));
            ptr::null_mut()
        }
    }
}

/// End a session.
///
/// Returns 0 on success, -1 on error.
#[no_mangle]
pub extern "C" fn cra_resolver_end_session(
    resolver: *mut CRAResolver,
    session_id: *const c_char,
) -> i32 {
    clear_error();

    let resolver = unsafe {
        match resolver.as_mut() {
            Some(r) => r,
            None => {
                set_error("Null resolver pointer".to_string());
                return -1;
            }
        }
    };

    let session_id_str = match unsafe { c_str_to_string(session_id) } {
        Some(s) => s,
        None => {
            set_error("Null or invalid session ID".to_string());
            return -1;
        }
    };

    match resolver.inner.end_session(&session_id_str) {
        Ok(()) => 0,
        Err(e) => {
            set_error(format!("Failed to end session: {}", e));
            -1
        }
    }
}

/// Resolve a CARP request.
///
/// Returns a JSON string containing the resolution on success, null on error.
/// The returned string must be freed with `cra_free_string`.
#[no_mangle]
pub extern "C" fn cra_resolver_resolve(
    resolver: *mut CRAResolver,
    session_id: *const c_char,
    agent_id: *const c_char,
    goal: *const c_char,
) -> *mut c_char {
    clear_error();

    let resolver = unsafe {
        match resolver.as_mut() {
            Some(r) => r,
            None => {
                set_error("Null resolver pointer".to_string());
                return ptr::null_mut();
            }
        }
    };

    let session_id_str = match unsafe { c_str_to_string(session_id) } {
        Some(s) => s,
        None => {
            set_error("Null or invalid session ID".to_string());
            return ptr::null_mut();
        }
    };

    let agent_id_str = match unsafe { c_str_to_string(agent_id) } {
        Some(s) => s,
        None => {
            set_error("Null or invalid agent ID".to_string());
            return ptr::null_mut();
        }
    };

    let goal_str = match unsafe { c_str_to_string(goal) } {
        Some(s) => s,
        None => {
            set_error("Null or invalid goal".to_string());
            return ptr::null_mut();
        }
    };

    let request = CARPRequest::new(session_id_str, agent_id_str, goal_str);

    match resolver.inner.resolve(&request) {
        Ok(resolution) => {
            match serde_json::to_string(&resolution) {
                Ok(json) => string_to_c(&json),
                Err(e) => {
                    set_error(format!("Failed to serialize resolution: {}", e));
                    ptr::null_mut()
                }
            }
        }
        Err(e) => {
            set_error(format!("Failed to resolve: {}", e));
            ptr::null_mut()
        }
    }
}

/// Execute an action.
///
/// Returns a JSON string containing the result on success, null on error.
/// The returned string must be freed with `cra_free_string`.
#[no_mangle]
pub extern "C" fn cra_resolver_execute(
    resolver: *mut CRAResolver,
    session_id: *const c_char,
    resolution_id: *const c_char,
    action_id: *const c_char,
    parameters_json: *const c_char,
) -> *mut c_char {
    clear_error();

    let resolver = unsafe {
        match resolver.as_mut() {
            Some(r) => r,
            None => {
                set_error("Null resolver pointer".to_string());
                return ptr::null_mut();
            }
        }
    };

    let session_id_str = match unsafe { c_str_to_string(session_id) } {
        Some(s) => s,
        None => {
            set_error("Null or invalid session ID".to_string());
            return ptr::null_mut();
        }
    };

    let resolution_id_str = match unsafe { c_str_to_string(resolution_id) } {
        Some(s) => s,
        None => {
            set_error("Null or invalid resolution ID".to_string());
            return ptr::null_mut();
        }
    };

    let action_id_str = match unsafe { c_str_to_string(action_id) } {
        Some(s) => s,
        None => {
            set_error("Null or invalid action ID".to_string());
            return ptr::null_mut();
        }
    };

    let params_str = match unsafe { c_str_to_string(parameters_json) } {
        Some(s) => s,
        None => "{}".to_string(),
    };

    let params: serde_json::Value = match serde_json::from_str(&params_str) {
        Ok(v) => v,
        Err(e) => {
            set_error(format!("Failed to parse parameters JSON: {}", e));
            return ptr::null_mut();
        }
    };

    match resolver.inner.execute(&session_id_str, &resolution_id_str, &action_id_str, params) {
        Ok(result) => {
            match serde_json::to_string(&result) {
                Ok(json) => string_to_c(&json),
                Err(e) => {
                    set_error(format!("Failed to serialize result: {}", e));
                    ptr::null_mut()
                }
            }
        }
        Err(e) => {
            set_error(format!("Failed to execute: {}", e));
            ptr::null_mut()
        }
    }
}

/// Get the trace for a session as JSONL.
///
/// Returns a JSONL string on success, null on error.
/// The returned string must be freed with `cra_free_string`.
#[no_mangle]
pub extern "C" fn cra_resolver_get_trace(
    resolver: *mut CRAResolver,
    session_id: *const c_char,
) -> *mut c_char {
    clear_error();

    let resolver = unsafe {
        match resolver.as_ref() {
            Some(r) => r,
            None => {
                set_error("Null resolver pointer".to_string());
                return ptr::null_mut();
            }
        }
    };

    let session_id_str = match unsafe { c_str_to_string(session_id) } {
        Some(s) => s,
        None => {
            set_error("Null or invalid session ID".to_string());
            return ptr::null_mut();
        }
    };

    match resolver.inner.get_trace(&session_id_str) {
        Ok(events) => {
            let lines: Vec<String> = events
                .iter()
                .filter_map(|e| serde_json::to_string(e).ok())
                .collect();
            string_to_c(&lines.join("\n"))
        }
        Err(e) => {
            set_error(format!("Failed to get trace: {}", e));
            ptr::null_mut()
        }
    }
}

/// Verify the hash chain for a session.
///
/// Returns a JSON string containing the verification result on success, null on error.
/// The returned string must be freed with `cra_free_string`.
#[no_mangle]
pub extern "C" fn cra_resolver_verify_chain(
    resolver: *mut CRAResolver,
    session_id: *const c_char,
) -> *mut c_char {
    clear_error();

    let resolver = unsafe {
        match resolver.as_ref() {
            Some(r) => r,
            None => {
                set_error("Null resolver pointer".to_string());
                return ptr::null_mut();
            }
        }
    };

    let session_id_str = match unsafe { c_str_to_string(session_id) } {
        Some(s) => s,
        None => {
            set_error("Null or invalid session ID".to_string());
            return ptr::null_mut();
        }
    };

    match resolver.inner.verify_chain(&session_id_str) {
        Ok(verification) => {
            match serde_json::to_string(&verification) {
                Ok(json) => string_to_c(&json),
                Err(e) => {
                    set_error(format!("Failed to serialize verification: {}", e));
                    ptr::null_mut()
                }
            }
        }
        Err(e) => {
            set_error(format!("Failed to verify chain: {}", e));
            ptr::null_mut()
        }
    }
}

// ============================================================================
// Version Info
// ============================================================================

/// Get the CRA core version.
///
/// Returns a static string (do not free).
#[no_mangle]
pub extern "C" fn cra_version() -> *const c_char {
    // This is a static string, safe to return directly
    b"0.1.0\0".as_ptr() as *const c_char
}

/// Get the CARP protocol version.
///
/// Returns a static string (do not free).
#[no_mangle]
pub extern "C" fn cra_carp_version() -> *const c_char {
    b"1.0\0".as_ptr() as *const c_char
}

/// Get the TRACE protocol version.
///
/// Returns a static string (do not free).
#[no_mangle]
pub extern "C" fn cra_trace_version() -> *const c_char {
    b"1.0\0".as_ptr() as *const c_char
}

/// Get the Atlas format version.
///
/// Returns a static string (do not free).
#[no_mangle]
pub extern "C" fn cra_atlas_version() -> *const c_char {
    b"1.0\0".as_ptr() as *const c_char
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_lifecycle() {
        // Create resolver
        let resolver = cra_resolver_new();
        assert!(!resolver.is_null());

        // Create session
        let agent_id = CString::new("test-agent").unwrap();
        let goal = CString::new("test goal").unwrap();
        let session_id = cra_resolver_create_session(resolver, agent_id.as_ptr(), goal.as_ptr());
        assert!(!session_id.is_null());

        // End session
        let result = cra_resolver_end_session(resolver, session_id);
        assert_eq!(result, 0);

        // Free
        cra_free_string(session_id);
        cra_resolver_free(resolver);
    }

    #[test]
    fn test_error_handling() {
        // Try to create session with null resolver
        let agent_id = CString::new("test").unwrap();
        let goal = CString::new("test").unwrap();
        let result = cra_resolver_create_session(ptr::null_mut(), agent_id.as_ptr(), goal.as_ptr());
        assert!(result.is_null());

        // Check error message
        let error = cra_get_last_error();
        assert!(!error.is_null());
        cra_free_string(error);
    }

    #[test]
    fn test_version_functions() {
        let version = cra_version();
        assert!(!version.is_null());
        let version_str = unsafe { CStr::from_ptr(version) }.to_str().unwrap();
        assert_eq!(version_str, "0.1.0");

        let carp_version = cra_carp_version();
        assert!(!carp_version.is_null());

        let trace_version = cra_trace_version();
        assert!(!trace_version.is_null());

        let atlas_version = cra_atlas_version();
        assert!(!atlas_version.is_null());
    }
}
