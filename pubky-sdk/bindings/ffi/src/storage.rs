//! FFI bindings for storage operations (SessionStorage and PublicStorage).

use std::os::raw::c_char;

use crate::error::{cstr_to_string, FfiBytesResult, FfiResult};
use crate::runtime::RUNTIME;

/// Opaque handle to SessionStorage (authenticated, as-me storage).
pub struct FfiSessionStorage(pub(crate) pubky::SessionStorage);

/// Opaque handle to PublicStorage (unauthenticated, read-only storage).
pub struct FfiPublicStorage(pub(crate) pubky::PublicStorage);

// --- SessionStorage operations ---

/// Get data from an absolute path (session storage).
/// Returns a result containing the response body as text.
///
/// # Safety
/// The storage and path pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_storage_get_text(
    storage: *const FfiSessionStorage,
    path: *const c_char,
) -> FfiResult {
    if storage.is_null() {
        return FfiResult::error("Null storage pointer".to_string(), -1);
    }

    let path = match cstr_to_string(path) {
        Some(s) => s,
        None => return FfiResult::error("Invalid path".to_string(), -1),
    };

    let storage = &(*storage).0;

    match RUNTIME.block_on(async {
        let resp = storage.get(&path).await?;
        resp.text().await.map_err(pubky::Error::from)
    }) {
        Ok(text) => FfiResult::success(text),
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// Get data from an absolute path as bytes (session storage).
/// Returns a result containing the response body as bytes.
///
/// # Safety
/// The storage and path pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_storage_get_bytes(
    storage: *const FfiSessionStorage,
    path: *const c_char,
) -> FfiBytesResult {
    if storage.is_null() {
        return FfiBytesResult::error("Null storage pointer".to_string(), -1);
    }

    let path = match cstr_to_string(path) {
        Some(s) => s,
        None => return FfiBytesResult::error("Invalid path".to_string(), -1),
    };

    let storage = &(*storage).0;

    match RUNTIME.block_on(async {
        let resp = storage.get(&path).await?;
        resp.bytes().await.map_err(pubky::Error::from)
    }) {
        Ok(bytes) => FfiBytesResult::success(bytes.to_vec()),
        Err(e) => FfiBytesResult::error(e.to_string(), 1),
    }
}

/// Get JSON data from an absolute path (session storage).
/// Returns a result containing the JSON string.
///
/// # Safety
/// The storage and path pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_storage_get_json(
    storage: *const FfiSessionStorage,
    path: *const c_char,
) -> FfiResult {
    if storage.is_null() {
        return FfiResult::error("Null storage pointer".to_string(), -1);
    }

    let path = match cstr_to_string(path) {
        Some(s) => s,
        None => return FfiResult::error("Invalid path".to_string(), -1),
    };

    let storage = &(*storage).0;

    match RUNTIME.block_on(storage.get_json::<_, serde_json::Value>(&path)) {
        Ok(value) => match serde_json::to_string(&value) {
            Ok(json_str) => FfiResult::success(json_str),
            Err(e) => FfiResult::error(e.to_string(), 1),
        },
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// Put text data at an absolute path (session storage).
/// Returns a result with success/error status.
///
/// # Safety
/// The storage, path, and body pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_storage_put_text(
    storage: *const FfiSessionStorage,
    path: *const c_char,
    body: *const c_char,
) -> FfiResult {
    if storage.is_null() {
        return FfiResult::error("Null storage pointer".to_string(), -1);
    }

    let path = match cstr_to_string(path) {
        Some(s) => s,
        None => return FfiResult::error("Invalid path".to_string(), -1),
    };

    let body = match cstr_to_string(body) {
        Some(s) => s,
        None => return FfiResult::error("Invalid body".to_string(), -1),
    };

    let storage = &(*storage).0;

    match RUNTIME.block_on(storage.put(&path, body)) {
        Ok(_) => FfiResult::success_empty(),
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// Put bytes at an absolute path (session storage).
/// Returns a result with success/error status.
///
/// # Arguments
/// * `body` - Pointer to byte data. Can be null only if `body_len` is 0 (empty content).
/// * `body_len` - Length of the byte data.
///
/// # Safety
/// The storage and path pointers must be valid.
/// If body_len > 0, body must point to valid memory of at least body_len bytes.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_storage_put_bytes(
    storage: *const FfiSessionStorage,
    path: *const c_char,
    body: *const u8,
    body_len: usize,
) -> FfiResult {
    if storage.is_null() {
        return FfiResult::error("Null storage pointer".to_string(), -1);
    }

    let path = match cstr_to_string(path) {
        Some(s) => s,
        None => return FfiResult::error("Invalid path".to_string(), -1),
    };

    // Allow null body only when body_len is 0 (writing empty content)
    if body.is_null() && body_len > 0 {
        return FfiResult::error("Null body pointer with non-zero length".to_string(), -1);
    }

    let body_bytes = if body.is_null() || body_len == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(body, body_len).to_vec()
    };

    let storage = &(*storage).0;

    match RUNTIME.block_on(storage.put(&path, body_bytes)) {
        Ok(_) => FfiResult::success_empty(),
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// Put JSON data at an absolute path (session storage).
/// Returns a result with success/error status.
///
/// # Safety
/// The storage, path, and json_body pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_storage_put_json(
    storage: *const FfiSessionStorage,
    path: *const c_char,
    json_body: *const c_char,
) -> FfiResult {
    if storage.is_null() {
        return FfiResult::error("Null storage pointer".to_string(), -1);
    }

    let path = match cstr_to_string(path) {
        Some(s) => s,
        None => return FfiResult::error("Invalid path".to_string(), -1),
    };

    let json_str = match cstr_to_string(json_body) {
        Some(s) => s,
        None => return FfiResult::error("Invalid JSON body".to_string(), -1),
    };

    let value: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => return FfiResult::error(format!("Invalid JSON: {}", e), -1),
    };

    let storage = &(*storage).0;

    match RUNTIME.block_on(storage.put_json(&path, &value)) {
        Ok(_) => FfiResult::success_empty(),
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// Delete data at an absolute path (session storage).
/// Returns a result with success/error status.
///
/// # Safety
/// The storage and path pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_storage_delete(
    storage: *const FfiSessionStorage,
    path: *const c_char,
) -> FfiResult {
    if storage.is_null() {
        return FfiResult::error("Null storage pointer".to_string(), -1);
    }

    let path = match cstr_to_string(path) {
        Some(s) => s,
        None => return FfiResult::error("Invalid path".to_string(), -1),
    };

    let storage = &(*storage).0;

    match RUNTIME.block_on(storage.delete(&path)) {
        Ok(_) => FfiResult::success_empty(),
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// Check if a path exists (session storage).
/// Returns a result with "true" or "false".
///
/// # Safety
/// The storage and path pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_storage_exists(
    storage: *const FfiSessionStorage,
    path: *const c_char,
) -> FfiResult {
    if storage.is_null() {
        return FfiResult::error("Null storage pointer".to_string(), -1);
    }

    let path = match cstr_to_string(path) {
        Some(s) => s,
        None => return FfiResult::error("Invalid path".to_string(), -1),
    };

    let storage = &(*storage).0;

    match RUNTIME.block_on(storage.exists(&path)) {
        Ok(exists) => FfiResult::success(if exists { "true" } else { "false" }.to_string()),
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// List directory contents (session storage).
/// Returns a result containing a JSON array of entry URLs.
///
/// # Safety
/// The storage and path pointers must be valid.
/// Path must end with '/'.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_storage_list(
    storage: *const FfiSessionStorage,
    path: *const c_char,
    limit: u16,
) -> FfiResult {
    if storage.is_null() {
        return FfiResult::error("Null storage pointer".to_string(), -1);
    }

    let path = match cstr_to_string(path) {
        Some(s) => s,
        None => return FfiResult::error("Invalid path".to_string(), -1),
    };

    let storage = &(*storage).0;

    match RUNTIME.block_on(async {
        let builder = storage.list(&path)?;
        let entries = builder.limit(limit).send().await?;
        let urls: Vec<String> = entries.iter().map(|e| e.to_pubky_url()).collect();
        Ok::<_, pubky::Error>(urls)
    }) {
        Ok(urls) => match serde_json::to_string(&urls) {
            Ok(json) => FfiResult::success(json),
            Err(e) => FfiResult::error(e.to_string(), 1),
        },
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// Free a session storage handle.
///
/// # Safety
/// The storage pointer must have been returned by a pubky FFI function.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_storage_free(storage: *mut FfiSessionStorage) {
    if !storage.is_null() {
        drop(Box::from_raw(storage));
    }
}

// --- PublicStorage operations ---

/// Get data from an addressed path (public storage).
/// Returns a result containing the response body as text.
///
/// # Safety
/// The storage and address pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_public_storage_get_text(
    storage: *const FfiPublicStorage,
    address: *const c_char,
) -> FfiResult {
    if storage.is_null() {
        return FfiResult::error("Null storage pointer".to_string(), -1);
    }

    let address = match cstr_to_string(address) {
        Some(s) => s,
        None => return FfiResult::error("Invalid address".to_string(), -1),
    };

    let storage = &(*storage).0;

    match RUNTIME.block_on(async {
        let resp = storage.get(&address).await?;
        resp.text().await.map_err(pubky::Error::from)
    }) {
        Ok(text) => FfiResult::success(text),
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// Get data from an addressed path as bytes (public storage).
/// Returns a result containing the response body as bytes.
///
/// # Safety
/// The storage and address pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_public_storage_get_bytes(
    storage: *const FfiPublicStorage,
    address: *const c_char,
) -> FfiBytesResult {
    if storage.is_null() {
        return FfiBytesResult::error("Null storage pointer".to_string(), -1);
    }

    let address = match cstr_to_string(address) {
        Some(s) => s,
        None => return FfiBytesResult::error("Invalid address".to_string(), -1),
    };

    let storage = &(*storage).0;

    match RUNTIME.block_on(async {
        let resp = storage.get(&address).await?;
        resp.bytes().await.map_err(pubky::Error::from)
    }) {
        Ok(bytes) => FfiBytesResult::success(bytes.to_vec()),
        Err(e) => FfiBytesResult::error(e.to_string(), 1),
    }
}

/// Get JSON data from an addressed path (public storage).
/// Returns a result containing the JSON string.
///
/// # Safety
/// The storage and address pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_public_storage_get_json(
    storage: *const FfiPublicStorage,
    address: *const c_char,
) -> FfiResult {
    if storage.is_null() {
        return FfiResult::error("Null storage pointer".to_string(), -1);
    }

    let address = match cstr_to_string(address) {
        Some(s) => s,
        None => return FfiResult::error("Invalid address".to_string(), -1),
    };

    let storage = &(*storage).0;

    match RUNTIME.block_on(storage.get_json::<_, serde_json::Value>(&address)) {
        Ok(value) => match serde_json::to_string(&value) {
            Ok(json_str) => FfiResult::success(json_str),
            Err(e) => FfiResult::error(e.to_string(), 1),
        },
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// Check if an addressed path exists (public storage).
/// Returns a result with "true" or "false".
///
/// # Safety
/// The storage and address pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_public_storage_exists(
    storage: *const FfiPublicStorage,
    address: *const c_char,
) -> FfiResult {
    if storage.is_null() {
        return FfiResult::error("Null storage pointer".to_string(), -1);
    }

    let address = match cstr_to_string(address) {
        Some(s) => s,
        None => return FfiResult::error("Invalid address".to_string(), -1),
    };

    let storage = &(*storage).0;

    match RUNTIME.block_on(storage.exists(&address)) {
        Ok(exists) => FfiResult::success(if exists { "true" } else { "false" }.to_string()),
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// List directory contents (public storage).
/// Returns a result containing a JSON array of entry URLs.
///
/// # Safety
/// The storage and address pointers must be valid.
/// Address must end with '/'.
#[no_mangle]
pub unsafe extern "C" fn pubky_public_storage_list(
    storage: *const FfiPublicStorage,
    address: *const c_char,
    limit: u16,
) -> FfiResult {
    if storage.is_null() {
        return FfiResult::error("Null storage pointer".to_string(), -1);
    }

    let address = match cstr_to_string(address) {
        Some(s) => s,
        None => return FfiResult::error("Invalid address".to_string(), -1),
    };

    let storage = &(*storage).0;

    match RUNTIME.block_on(async {
        let builder = storage.list(&address)?;
        let entries = builder.limit(limit).send().await?;
        let urls: Vec<String> = entries.iter().map(|e| e.to_pubky_url()).collect();
        Ok::<_, pubky::Error>(urls)
    }) {
        Ok(urls) => match serde_json::to_string(&urls) {
            Ok(json) => FfiResult::success(json),
            Err(e) => FfiResult::error(e.to_string(), 1),
        },
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// Free a public storage handle.
///
/// # Safety
/// The storage pointer must have been returned by a pubky FFI function.
#[no_mangle]
pub unsafe extern "C" fn pubky_public_storage_free(storage: *mut FfiPublicStorage) {
    if !storage.is_null() {
        drop(Box::from_raw(storage));
    }
}
