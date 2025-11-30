//! FFI bindings for the Pubky SDK.
//!
//! This module provides C-compatible FFI functions that expose the Pubky SDK functionality
//! for integration with other languages like Ruby, Python, etc.
//!
//! # Design
//!
//! - **Singleton pattern**: The `HttpClient` and `Pubky` facade are lazy-initialized on first use.
//! - **Blocking API**: All async functions are exposed as blocking via `RUNTIME.block_on()`.
//! - **Memory management**: Strings returned to the caller are heap-allocated C strings that
//!   must be freed by calling `pubky_string_free()`.
//!
//! # Thread Safety
//!
//! The singleton instances are protected by `Mutex` and are thread-safe.

#![allow(clippy::missing_panics_doc, reason = "FFI boundary functions")]
#![allow(clippy::not_unsafe_ptr_arg_deref, reason = "FFI boundary with documented safety")]

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{LazyLock, Mutex};

use pubky::{Capabilities, Keypair, Method, PublicKey, Pubky, PubkyHttpClient};
use tokio::runtime::Runtime;
use url::Url;

/// Global Tokio runtime for executing async operations synchronously.
static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime")
});

/// Global Pubky facade instance (mainnet).
static PUBKY_INSTANCE: LazyLock<Mutex<Option<Pubky>>> = LazyLock::new(|| Mutex::new(None));

/// Global Pubky facade instance (testnet).
static PUBKY_TESTNET_INSTANCE: LazyLock<Mutex<Option<Pubky>>> = LazyLock::new(|| Mutex::new(None));

/// Global HTTP client instance (mainnet).
static HTTP_CLIENT_INSTANCE: LazyLock<Mutex<Option<PubkyHttpClient>>> =
    LazyLock::new(|| Mutex::new(None));

/// Global HTTP client instance (testnet).
static HTTP_CLIENT_TESTNET_INSTANCE: LazyLock<Mutex<Option<PubkyHttpClient>>> =
    LazyLock::new(|| Mutex::new(None));

// ============================================================================
// Helper functions
// ============================================================================

/// Get or initialize the mainnet Pubky instance.
fn get_pubky() -> Result<Pubky, String> {
    let mut guard = PUBKY_INSTANCE.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        *guard = Some(Pubky::new().map_err(|e| e.to_string())?);
    }
    guard
        .as_ref()
        .cloned()
        .ok_or_else(|| "Failed to initialize Pubky instance".to_string())
}

/// Get or initialize the testnet Pubky instance.
fn get_pubky_testnet() -> Result<Pubky, String> {
    let mut guard = PUBKY_TESTNET_INSTANCE.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        *guard = Some(Pubky::testnet().map_err(|e| e.to_string())?);
    }
    guard
        .as_ref()
        .cloned()
        .ok_or_else(|| "Failed to initialize Pubky testnet instance".to_string())
}

/// Get or initialize the mainnet HTTP client.
fn get_http_client() -> Result<PubkyHttpClient, String> {
    let mut guard = HTTP_CLIENT_INSTANCE.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        *guard = Some(PubkyHttpClient::new().map_err(|e| e.to_string())?);
    }
    guard
        .as_ref()
        .cloned()
        .ok_or_else(|| "Failed to initialize HTTP client".to_string())
}

/// Get or initialize the testnet HTTP client.
fn get_http_client_testnet() -> Result<PubkyHttpClient, String> {
    let mut guard = HTTP_CLIENT_TESTNET_INSTANCE
        .lock()
        .map_err(|e| e.to_string())?;
    if guard.is_none() {
        *guard = Some(PubkyHttpClient::testnet().map_err(|e| e.to_string())?);
    }
    guard
        .as_ref()
        .cloned()
        .ok_or_else(|| "Failed to initialize HTTP testnet client".to_string())
}

/// Convert a C string pointer to a Rust string, returning an error message if invalid.
///
/// # Safety
///
/// The caller must ensure `ptr` is a valid, null-terminated C string or null.
unsafe fn c_str_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: Caller guarantees ptr is valid and null-terminated.
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .ok()
        .map(String::from)
}

/// Convert a C string pointer to a Rust string, returning the provided default if null/invalid.
///
/// # Safety
///
/// The caller must ensure `ptr` is a valid, null-terminated C string or null.
unsafe fn c_str_to_string_or(ptr: *const c_char, default: &str) -> String {
    // SAFETY: Caller guarantees ptr is valid or null.
    unsafe { c_str_to_string(ptr) }.unwrap_or_else(|| default.to_string())
}

/// Allocate a C string from a Rust string. The caller must free this with `pubky_string_free`.
///
/// If the string contains interior null bytes (which shouldn't happen with JSON),
/// returns an error JSON response instead.
fn string_to_c(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cstring) => cstring.into_raw(),
        Err(_) => {
            // The string contains interior null bytes - this shouldn't happen with JSON
            // Return a simple error message that won't have null bytes
            CString::new(r#"{"success":false,"error":"String contains invalid null bytes"}"#)
                .expect("Error message should not contain null bytes")
                .into_raw()
        }
    }
}

/// Create a JSON error response.
fn error_response(error: &str) -> *mut c_char {
    let json = serde_json::json!({
        "success": false,
        "error": error
    });
    string_to_c(&json.to_string())
}

/// Create a JSON success response with data.
fn success_response(data: &serde_json::Value) -> *mut c_char {
    let json = serde_json::json!({
        "success": true,
        "data": data
    });
    string_to_c(&json.to_string())
}

/// Create a simple JSON success response.
fn success_simple() -> *mut c_char {
    success_response(&serde_json::json!(null))
}

/// Format capabilities slice as a string.
fn format_capabilities(caps: &[pubky::Capability]) -> String {
    caps.iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

// ============================================================================
// Memory Management
// ============================================================================

/// Free a string that was allocated by this library.
///
/// # Safety
///
/// The `ptr` must be a valid pointer returned by one of the FFI functions in this library,
/// or null (which is a no-op).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubky_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: Caller guarantees ptr was allocated by this library.
        drop(unsafe { CString::from_raw(ptr) });
    }
}

// ============================================================================
// Keypair Operations
// ============================================================================

/// Generate a new random keypair and return it as a JSON string.
///
/// Returns a JSON object with `success: true` and `data` containing:
/// - `secret_key`: hex-encoded 32-byte secret key
/// - `public_key`: z32-encoded public key
///
/// The returned string must be freed with `pubky_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn pubky_keypair_random() -> *mut c_char {
    let keypair = Keypair::random();
    let secret_hex = hex::encode(keypair.secret_key());
    let public_key = keypair.public_key().to_string();

    success_response(&serde_json::json!({
        "secret_key": secret_hex,
        "public_key": public_key
    }))
}

/// Create a keypair from a hex-encoded secret key.
///
/// # Safety
///
/// `secret_key_hex` must be a valid null-terminated C string containing a 64-character hex string.
///
/// Returns a JSON object with the public key or an error.
/// The returned string must be freed with `pubky_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubky_keypair_from_secret_key(
    secret_key_hex: *const c_char,
) -> *mut c_char {
    // SAFETY: Caller guarantees secret_key_hex is valid.
    let Some(hex_str) = (unsafe { c_str_to_string(secret_key_hex) }) else {
        return error_response("Invalid secret key string");
    };

    let Ok(secret_bytes) = hex::decode(&hex_str) else {
        return error_response("Invalid hex encoding for secret key");
    };

    let secret_array: [u8; 32] = match secret_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => return error_response("Secret key must be exactly 32 bytes"),
    };

    let keypair = Keypair::from_secret_key(&secret_array);
    let public_key = keypair.public_key().to_string();

    success_response(&serde_json::json!({
        "public_key": public_key
    }))
}

// ============================================================================
// Sign Up / Sign In
// ============================================================================

/// Sign up a new user on a homeserver.
///
/// # Safety
///
/// All string parameters must be valid null-terminated C strings or null.
///
/// # Parameters
/// - `secret_key_hex`: 64-character hex-encoded secret key
/// - `homeserver_pubkey`: z32-encoded public key of the homeserver
/// - `signup_token`: optional signup token (can be null)
/// - `testnet`: if non-zero, use testnet configuration
///
/// Returns JSON with session info or error.
/// The returned string must be freed with `pubky_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubky_signup(
    secret_key_hex: *const c_char,
    homeserver_pubkey: *const c_char,
    signup_token: *const c_char,
    testnet: i32,
) -> *mut c_char {
    // SAFETY: Caller guarantees pointers are valid.
    let Some(secret_hex) = (unsafe { c_str_to_string(secret_key_hex) }) else {
        return error_response("Invalid secret key");
    };

    // SAFETY: Caller guarantees pointers are valid.
    let Some(homeserver_str) = (unsafe { c_str_to_string(homeserver_pubkey) }) else {
        return error_response("Invalid homeserver public key");
    };

    // SAFETY: Caller guarantees pointers are valid.
    let token = unsafe { c_str_to_string(signup_token) };

    let Ok(secret_bytes) = hex::decode(&secret_hex) else {
        return error_response("Invalid hex encoding for secret key");
    };

    let secret_array: [u8; 32] = match secret_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => return error_response("Secret key must be exactly 32 bytes"),
    };

    let keypair = Keypair::from_secret_key(&secret_array);

    let homeserver = match PublicKey::try_from(homeserver_str.as_str()) {
        Ok(pk) => pk,
        Err(e) => return error_response(&format!("Invalid homeserver public key: {e}")),
    };

    let pubky = if testnet != 0 {
        match get_pubky_testnet() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    } else {
        match get_pubky() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    };

    let signer = pubky.signer(keypair);

    let result = RUNTIME.block_on(async { signer.signup(&homeserver, token.as_deref()).await });

    match result {
        Ok(session) => {
            let info = session.info();
            success_response(&serde_json::json!({
                "public_key": info.public_key().to_string(),
                "capabilities": format_capabilities(info.capabilities()),
            }))
        }
        Err(e) => error_response(&e.to_string()),
    }
}

/// Sign in an existing user.
///
/// # Safety
///
/// `secret_key_hex` must be a valid null-terminated C string.
///
/// # Parameters
/// - `secret_key_hex`: 64-character hex-encoded secret key
/// - `testnet`: if non-zero, use testnet configuration
///
/// Returns JSON with session info or error.
/// The returned string must be freed with `pubky_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubky_signin(secret_key_hex: *const c_char, testnet: i32) -> *mut c_char {
    // SAFETY: Caller guarantees secret_key_hex is valid.
    let Some(secret_hex) = (unsafe { c_str_to_string(secret_key_hex) }) else {
        return error_response("Invalid secret key");
    };

    let Ok(secret_bytes) = hex::decode(&secret_hex) else {
        return error_response("Invalid hex encoding for secret key");
    };

    let secret_array: [u8; 32] = match secret_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => return error_response("Secret key must be exactly 32 bytes"),
    };

    let keypair = Keypair::from_secret_key(&secret_array);

    let pubky = if testnet != 0 {
        match get_pubky_testnet() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    } else {
        match get_pubky() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    };

    let signer = pubky.signer(keypair);

    let result = RUNTIME.block_on(async { signer.signin().await });

    match result {
        Ok(session) => {
            let info = session.info();
            success_response(&serde_json::json!({
                "public_key": info.public_key().to_string(),
                "capabilities": format_capabilities(info.capabilities()),
            }))
        }
        Err(e) => error_response(&e.to_string()),
    }
}

// ============================================================================
// Storage Operations
// ============================================================================

/// Put data to a path in the user's storage.
///
/// # Safety
///
/// All string parameters must be valid null-terminated C strings.
///
/// # Parameters
/// - `secret_key_hex`: 64-character hex-encoded secret key
/// - `path`: absolute path (e.g., "/pub/my-app/file.txt")
/// - `content`: content to store
/// - `testnet`: if non-zero, use testnet configuration
///
/// Returns JSON with success status or error.
/// The returned string must be freed with `pubky_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubky_put(
    secret_key_hex: *const c_char,
    path: *const c_char,
    content: *const c_char,
    testnet: i32,
) -> *mut c_char {
    // SAFETY: Caller guarantees pointers are valid.
    let Some(secret_hex) = (unsafe { c_str_to_string(secret_key_hex) }) else {
        return error_response("Invalid secret key");
    };

    // SAFETY: Caller guarantees pointers are valid.
    let Some(path_str) = (unsafe { c_str_to_string(path) }) else {
        return error_response("Invalid path");
    };

    // SAFETY: Caller guarantees pointers are valid.
    let Some(content_str) = (unsafe { c_str_to_string(content) }) else {
        return error_response("Invalid content");
    };

    let Ok(secret_bytes) = hex::decode(&secret_hex) else {
        return error_response("Invalid hex encoding for secret key");
    };

    let secret_array: [u8; 32] = match secret_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => return error_response("Secret key must be exactly 32 bytes"),
    };

    let keypair = Keypair::from_secret_key(&secret_array);

    let pubky = if testnet != 0 {
        match get_pubky_testnet() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    } else {
        match get_pubky() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    };

    let signer = pubky.signer(keypair);

    let result = RUNTIME.block_on(async {
        let session = signer.signin().await?;
        session.storage().put(&path_str, content_str).await
    });

    match result {
        Ok(_) => success_simple(),
        Err(e) => error_response(&e.to_string()),
    }
}

/// Get data from a path in the user's storage.
///
/// # Safety
///
/// All string parameters must be valid null-terminated C strings.
///
/// # Parameters
/// - `secret_key_hex`: 64-character hex-encoded secret key
/// - `path`: absolute path (e.g., "/pub/my-app/file.txt")
/// - `testnet`: if non-zero, use testnet configuration
///
/// Returns JSON with content or error.
/// The returned string must be freed with `pubky_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubky_get(
    secret_key_hex: *const c_char,
    path: *const c_char,
    testnet: i32,
) -> *mut c_char {
    // SAFETY: Caller guarantees pointers are valid.
    let Some(secret_hex) = (unsafe { c_str_to_string(secret_key_hex) }) else {
        return error_response("Invalid secret key");
    };

    // SAFETY: Caller guarantees pointers are valid.
    let Some(path_str) = (unsafe { c_str_to_string(path) }) else {
        return error_response("Invalid path");
    };

    let Ok(secret_bytes) = hex::decode(&secret_hex) else {
        return error_response("Invalid hex encoding for secret key");
    };

    let secret_array: [u8; 32] = match secret_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => return error_response("Secret key must be exactly 32 bytes"),
    };

    let keypair = Keypair::from_secret_key(&secret_array);

    let pubky = if testnet != 0 {
        match get_pubky_testnet() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    } else {
        match get_pubky() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    };

    let signer = pubky.signer(keypair);

    let result = RUNTIME.block_on(async {
        let session = signer.signin().await?;
        let response = session.storage().get(&path_str).await?;
        response.text().await.map_err(pubky::Error::from)
    });

    match result {
        Ok(content) => success_response(&serde_json::json!({
            "content": content
        })),
        Err(e) => error_response(&e.to_string()),
    }
}

/// Delete data at a path in the user's storage.
///
/// # Safety
///
/// All string parameters must be valid null-terminated C strings.
///
/// # Parameters
/// - `secret_key_hex`: 64-character hex-encoded secret key
/// - `path`: absolute path (e.g., "/pub/my-app/file.txt")
/// - `testnet`: if non-zero, use testnet configuration
///
/// Returns JSON with success status or error.
/// The returned string must be freed with `pubky_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubky_delete(
    secret_key_hex: *const c_char,
    path: *const c_char,
    testnet: i32,
) -> *mut c_char {
    // SAFETY: Caller guarantees pointers are valid.
    let Some(secret_hex) = (unsafe { c_str_to_string(secret_key_hex) }) else {
        return error_response("Invalid secret key");
    };

    // SAFETY: Caller guarantees pointers are valid.
    let Some(path_str) = (unsafe { c_str_to_string(path) }) else {
        return error_response("Invalid path");
    };

    let Ok(secret_bytes) = hex::decode(&secret_hex) else {
        return error_response("Invalid hex encoding for secret key");
    };

    let secret_array: [u8; 32] = match secret_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => return error_response("Secret key must be exactly 32 bytes"),
    };

    let keypair = Keypair::from_secret_key(&secret_array);

    let pubky = if testnet != 0 {
        match get_pubky_testnet() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    } else {
        match get_pubky() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    };

    let signer = pubky.signer(keypair);

    let result = RUNTIME.block_on(async {
        let session = signer.signin().await?;
        session.storage().delete(&path_str).await
    });

    match result {
        Ok(_) => success_simple(),
        Err(e) => error_response(&e.to_string()),
    }
}

// ============================================================================
// Public Storage Operations (Read-only, no authentication)
// ============================================================================

/// Get public data from any user's storage.
///
/// # Safety
///
/// `address` must be a valid null-terminated C string.
///
/// # Parameters
/// - `address`: addressed resource (e.g., "pubky<pk>/pub/my-app/file.txt" or "<pk>/pub/...")
/// - `testnet`: if non-zero, use testnet configuration
///
/// Returns JSON with content or error.
/// The returned string must be freed with `pubky_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubky_public_get(address: *const c_char, testnet: i32) -> *mut c_char {
    // SAFETY: Caller guarantees address is valid.
    let Some(addr_str) = (unsafe { c_str_to_string(address) }) else {
        return error_response("Invalid address");
    };

    let pubky = if testnet != 0 {
        match get_pubky_testnet() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    } else {
        match get_pubky() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    };

    let result = RUNTIME.block_on(async {
        let response = pubky.public_storage().get(&addr_str).await?;
        response.text().await.map_err(pubky::Error::from)
    });

    match result {
        Ok(content) => success_response(&serde_json::json!({
            "content": content
        })),
        Err(e) => error_response(&e.to_string()),
    }
}

/// List entries in a public directory.
///
/// # Safety
///
/// `address` must be a valid null-terminated C string ending with '/'.
///
/// # Parameters
/// - `address`: addressed directory (e.g., "pubky<pk>/pub/my-app/")
/// - `limit`: maximum number of entries to return (0 for default)
/// - `testnet`: if non-zero, use testnet configuration
///
/// Returns JSON with list of entries or error.
/// The returned string must be freed with `pubky_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubky_public_list(
    address: *const c_char,
    limit: u16,
    testnet: i32,
) -> *mut c_char {
    // SAFETY: Caller guarantees address is valid.
    let Some(addr_str) = (unsafe { c_str_to_string(address) }) else {
        return error_response("Invalid address");
    };

    let pubky = if testnet != 0 {
        match get_pubky_testnet() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    } else {
        match get_pubky() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    };

    let result = RUNTIME.block_on(async {
        let public_storage = pubky.public_storage();
        let mut builder = public_storage.list(&addr_str)?;
        if limit > 0 {
            builder = builder.limit(limit);
        }
        builder.send().await
    });

    match result {
        Ok(entries) => {
            let items: Vec<serde_json::Value> = entries
                .iter()
                .map(|e| {
                    let url_str = e.to_pubky_url();
                    let is_dir = e.path.as_str().ends_with('/');
                    serde_json::json!({
                        "url": url_str,
                        "path": e.path.as_str(),
                        "owner": e.owner.to_string(),
                        "is_dir": is_dir,
                    })
                })
                .collect();
            success_response(&serde_json::json!({
                "entries": items
            }))
        }
        Err(e) => error_response(&e.to_string()),
    }
}

// ============================================================================
// HTTP Client Operations
// ============================================================================

/// Make an HTTP request using the Pubky HTTP client.
///
/// This exposes low-level HTTP functionality similar to the Rust example:
/// ```rust,ignore
/// let mut rb = client.request(args.method.clone(), &args.url);
/// ```
///
/// # Safety
///
/// All string parameters must be valid null-terminated C strings.
///
/// # Parameters
/// - `method`: HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
/// - `url`: URL to request (supports pubky:// and https:// schemes)
/// - `body`: optional request body (can be null)
/// - `headers_json`: optional JSON object with headers (can be null)
/// - `testnet`: if non-zero, use testnet configuration
///
/// Returns JSON with response status, headers, and body.
/// The returned string must be freed with `pubky_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubky_http_request(
    method: *const c_char,
    url: *const c_char,
    body: *const c_char,
    headers_json: *const c_char,
    testnet: i32,
) -> *mut c_char {
    // SAFETY: Caller guarantees pointers are valid.
    let Some(method_str) = (unsafe { c_str_to_string(method) }) else {
        return error_response("Invalid method");
    };

    // SAFETY: Caller guarantees pointers are valid.
    let Some(url_str) = (unsafe { c_str_to_string(url) }) else {
        return error_response("Invalid URL");
    };

    // SAFETY: Caller guarantees pointers are valid.
    let body_opt = unsafe { c_str_to_string(body) };
    // SAFETY: Caller guarantees pointers are valid.
    let headers_opt = unsafe { c_str_to_string(headers_json) };

    let http_method = match method_str.to_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "PATCH" => Method::PATCH,
        "HEAD" => Method::HEAD,
        "OPTIONS" => Method::OPTIONS,
        _ => return error_response(&format!("Unsupported HTTP method: {method_str}")),
    };

    let parsed_url = match Url::parse(&url_str) {
        Ok(u) => u,
        Err(e) => return error_response(&format!("Invalid URL: {e}")),
    };

    let client = if testnet != 0 {
        match get_http_client_testnet() {
            Ok(c) => c,
            Err(e) => return error_response(&e),
        }
    } else {
        match get_http_client() {
            Ok(c) => c,
            Err(e) => return error_response(&e),
        }
    };

    let result = RUNTIME.block_on(async {
        let mut rb = client.request(http_method, &parsed_url);

        // Apply headers if provided
        if let Some(headers_str) = headers_opt
            && let Ok(headers) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
                &headers_str,
            )
        {
            for (key, value) in headers {
                if let Some(v) = value.as_str() {
                    rb = rb.header(key.as_str(), v);
                }
            }
        }

        // Apply body if provided
        if let Some(body_str) = body_opt {
            rb = rb.body(body_str);
        }

        let response = rb.send().await.map_err(pubky::Error::from)?;

        let status = response.status().as_u16();
        let headers: serde_json::Map<String, serde_json::Value> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.into())))
            .collect();

        let body = response.text().await.map_err(pubky::Error::from)?;

        Ok::<_, pubky::Error>(serde_json::json!({
            "status": status,
            "headers": headers,
            "body": body
        }))
    });

    match result {
        Ok(ref data) => success_response(data),
        Err(e) => error_response(&e.to_string()),
    }
}

// ============================================================================
// PKDNS Operations
// ============================================================================

/// Resolve the homeserver for a given public key.
///
/// # Safety
///
/// `public_key` must be a valid null-terminated C string.
///
/// # Parameters
/// - `public_key`: z32-encoded public key
/// - `testnet`: if non-zero, use testnet configuration
///
/// Returns JSON with homeserver public key or null if not found.
/// The returned string must be freed with `pubky_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubky_resolve_homeserver(
    public_key: *const c_char,
    testnet: i32,
) -> *mut c_char {
    // SAFETY: Caller guarantees public_key is valid.
    let Some(pk_str) = (unsafe { c_str_to_string(public_key) }) else {
        return error_response("Invalid public key");
    };

    let pk = match PublicKey::try_from(pk_str.as_str()) {
        Ok(pk) => pk,
        Err(e) => return error_response(&format!("Invalid public key: {e}")),
    };

    let pubky = if testnet != 0 {
        match get_pubky_testnet() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    } else {
        match get_pubky() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    };

    let result = RUNTIME.block_on(async { pubky.get_homeserver_of(&pk).await });

    match result {
        Some(homeserver) => success_response(&serde_json::json!({
            "homeserver": homeserver.to_string()
        })),
        None => success_response(&serde_json::json!({
            "homeserver": null
        })),
    }
}

// ============================================================================
// Auth Flow Operations
// ============================================================================

/// Start a Pubky authentication flow for keyless/QR apps.
///
/// # Safety
///
/// `capabilities` must be a valid null-terminated C string.
///
/// # Parameters
/// - `capabilities`: comma-separated capability string (e.g., "/pub/app/:rw,/pub/foo/:r")
/// - `testnet`: if non-zero, use testnet configuration
///
/// Returns JSON with authorization URL.
/// The returned string must be freed with `pubky_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubky_auth_start(capabilities: *const c_char, testnet: i32) -> *mut c_char {
    // SAFETY: Caller guarantees capabilities is valid.
    let caps_str = unsafe { c_str_to_string_or(capabilities, "") };

    let caps = match Capabilities::try_from(caps_str.as_str()) {
        Ok(c) => c,
        Err(e) => return error_response(&format!("Invalid capabilities: {e}")),
    };

    let pubky = if testnet != 0 {
        match get_pubky_testnet() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    } else {
        match get_pubky() {
            Ok(p) => p,
            Err(e) => return error_response(&e),
        }
    };

    let flow = match pubky.start_auth_flow(&caps) {
        Ok(f) => f,
        Err(e) => return error_response(&format!("Failed to start auth flow: {e}")),
    };

    success_response(&serde_json::json!({
        "authorization_url": flow.authorization_url().to_string()
    }))
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Get the version of the pubky-sdk-ffi library.
///
/// Returns a C string with the version. The returned string must be freed with `pubky_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn pubky_version() -> *mut c_char {
    string_to_c(env!("CARGO_PKG_VERSION"))
}

/// Resolve a pubky address to its transport URL.
///
/// # Safety
///
/// `address` must be a valid null-terminated C string.
///
/// # Parameters
/// - `address`: pubky address (e.g., "pubky<pk>/pub/app/file" or "pubky://<pk>/...")
///
/// Returns JSON with the resolved HTTPS URL.
/// The returned string must be freed with `pubky_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubky_resolve_address(address: *const c_char) -> *mut c_char {
    // SAFETY: Caller guarantees address is valid.
    let Some(addr_str) = (unsafe { c_str_to_string(address) }) else {
        return error_response("Invalid address");
    };

    match pubky::resolve_pubky(&addr_str) {
        Ok(url) => success_response(&serde_json::json!({
            "url": url.to_string()
        })),
        Err(e) => error_response(&format!("Failed to resolve address: {e}")),
    }
}
