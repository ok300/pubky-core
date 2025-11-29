//! FFI-safe error handling.
//!
//! Provides error types and functions for handling errors across FFI boundaries.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// FFI-safe result structure.
#[repr(C)]
pub struct FfiResult {
    /// Pointer to the result data (if success), null otherwise.
    pub data: *mut c_char,
    /// Error message (if error), null otherwise.
    pub error: *mut c_char,
    /// 0 for success, non-zero error code otherwise.
    pub code: i32,
}

impl FfiResult {
    /// Create a successful result with string data.
    pub fn success(data: String) -> Self {
        let c_string = CString::new(data).unwrap_or_else(|_| CString::new("").unwrap());
        Self {
            data: c_string.into_raw(),
            error: ptr::null_mut(),
            code: 0,
        }
    }

    /// Create a successful result with no data.
    pub fn success_empty() -> Self {
        Self {
            data: ptr::null_mut(),
            error: ptr::null_mut(),
            code: 0,
        }
    }

    /// Create an error result.
    pub fn error(message: String, code: i32) -> Self {
        let c_string =
            CString::new(message).unwrap_or_else(|_| CString::new("Unknown error").unwrap());
        Self {
            data: ptr::null_mut(),
            error: c_string.into_raw(),
            code,
        }
    }

    /// Create an error from a pubky Error.
    pub fn from_pubky_error(err: pubky::Error) -> Self {
        let code = match &err {
            pubky::Error::Request(_) => 1,
            pubky::Error::Pkarr(_) => 2,
            pubky::Error::Parse(_) => 3,
            pubky::Error::Authentication(_) => 4,
            pubky::Error::Build(_) => 5,
        };
        Self::error(err.to_string(), code)
    }
}

/// FFI-safe result for binary data.
#[repr(C)]
pub struct FfiBytesResult {
    /// Pointer to the result data (if success), null otherwise.
    pub data: *mut u8,
    /// Length of the data.
    pub len: usize,
    /// Error message (if error), null otherwise.
    pub error: *mut c_char,
    /// 0 for success, non-zero error code otherwise.
    pub code: i32,
}

impl FfiBytesResult {
    /// Create a successful result with byte data.
    ///
    /// Note: We use `into_boxed_slice()` which shrinks capacity to exactly match
    /// length, so when freeing with `Vec::from_raw_parts(ptr, len, len)`, the
    /// capacity will be correct.
    pub fn success(data: Vec<u8>) -> Self {
        let len = data.len();
        let boxed = data.into_boxed_slice();
        let ptr = Box::into_raw(boxed) as *mut u8;
        Self {
            data: ptr,
            len,
            error: ptr::null_mut(),
            code: 0,
        }
    }

    /// Create an error result.
    pub fn error(message: String, code: i32) -> Self {
        let c_string =
            CString::new(message).unwrap_or_else(|_| CString::new("Unknown error").unwrap());
        Self {
            data: ptr::null_mut(),
            len: 0,
            error: c_string.into_raw(),
            code,
        }
    }
}

/// Free a string returned by FFI functions.
///
/// # Safety
/// The pointer must have been returned by a pubky FFI function.
#[no_mangle]
pub unsafe extern "C" fn pubky_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

/// Free bytes returned by FFI functions.
///
/// # Safety
/// The pointer and length must have been returned by a pubky FFI function.
/// The bytes must have been allocated with `into_boxed_slice()` which ensures
/// capacity equals length.
#[no_mangle]
pub unsafe extern "C" fn pubky_bytes_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        // Safe: FfiBytesResult::success uses into_boxed_slice() which ensures capacity == len
        let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, len));
    }
}

/// Free an FfiResult structure.
///
/// # Safety
/// The result must have been returned by a pubky FFI function.
#[no_mangle]
pub unsafe extern "C" fn pubky_result_free(result: FfiResult) {
    if !result.data.is_null() {
        drop(CString::from_raw(result.data));
    }
    if !result.error.is_null() {
        drop(CString::from_raw(result.error));
    }
}

/// Free an FfiBytesResult structure.
///
/// # Safety
/// The result must have been returned by a pubky FFI function.
#[no_mangle]
pub unsafe extern "C" fn pubky_bytes_result_free(result: FfiBytesResult) {
    if !result.data.is_null() {
        // Safe: FfiBytesResult::success uses into_boxed_slice() which ensures capacity == len
        let _ = Box::from_raw(std::slice::from_raw_parts_mut(result.data, result.len));
    }
    if !result.error.is_null() {
        drop(CString::from_raw(result.error));
    }
}

/// Helper to convert a C string to a Rust String.
///
/// # Safety
/// The pointer must be a valid null-terminated C string.
pub(crate) unsafe fn cstr_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok().map(String::from)
}
