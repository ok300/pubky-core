//! FFI bindings for the PubkyHttpClient.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

use pubky::{Method, PubkyHttpClient};

use crate::error::{FfiBytesResult, FfiResult};
use crate::runtime::RUNTIME;

/// Opaque handle to a PubkyHttpClient.
pub struct FfiHttpClient(pub(crate) PubkyHttpClient);

/// Safely convert a C string pointer to a Rust String.
/// Returns None if the pointer is null or if the string is invalid UTF-8.
unsafe fn safe_cstr_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    match CStr::from_ptr(ptr).to_str() {
        Ok(s) => Some(s.to_string()),
        Err(_) => None,
    }
}

/// Create a new PubkyHttpClient with mainnet defaults.
/// Returns a pointer to the client, or null on failure.
/// The caller must free the client with `pubky_http_client_free`.
#[no_mangle]
pub extern "C" fn pubky_http_client_new() -> *mut FfiHttpClient {
    match PubkyHttpClient::new() {
        Ok(client) => Box::into_raw(Box::new(FfiHttpClient(client))),
        Err(_) => ptr::null_mut(),
    }
}

/// Create a PubkyHttpClient preconfigured for a local testnet.
/// Returns a pointer to the client, or null on failure.
/// The caller must free the client with `pubky_http_client_free`.
#[no_mangle]
pub extern "C" fn pubky_http_client_testnet() -> *mut FfiHttpClient {
    match PubkyHttpClient::testnet() {
        Ok(client) => Box::into_raw(Box::new(FfiHttpClient(client))),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a PubkyHttpClient.
///
/// # Safety
/// The client pointer must have been returned by a pubky FFI function.
#[no_mangle]
pub unsafe extern "C" fn pubky_http_client_free(client: *mut FfiHttpClient) {
    if !client.is_null() {
        drop(Box::from_raw(client));
    }
}

/// Perform an HTTP request using the PubkyHttpClient.
///
/// This method can make requests to:
/// 1. Standard HTTPS URLs
/// 2. HTTPS URLs with a pkarr PublicKey as top-level domain
/// 3. `_pubky.<public-key>` URLs
///
/// Returns a result containing the response body as text.
///
/// # Arguments
/// * `client` - Pointer to the PubkyHttpClient
/// * `method` - HTTP method string (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
/// * `url` - The URL to request (can be pubky:// or https://)
/// * `body` - Optional request body (can be null)
/// * `headers` - Optional headers as JSON object string (can be null), e.g. `{"Content-Type": "application/json"}`
///
/// # Safety
/// The client pointer must be valid. Method and url must be valid C strings or null.
#[no_mangle]
pub unsafe extern "C" fn pubky_http_client_request(
    client: *const FfiHttpClient,
    method: *const c_char,
    url: *const c_char,
    body: *const c_char,
    headers: *const c_char,
) -> FfiResult {
    if client.is_null() {
        return FfiResult::error("Null client pointer".to_string(), -1);
    }

    let method_str = match safe_cstr_to_string(method) {
        Some(s) => s,
        None => return FfiResult::error("Invalid or null method".to_string(), -1),
    };

    let url_str = match safe_cstr_to_string(url) {
        Some(s) => s,
        None => return FfiResult::error("Invalid or null URL".to_string(), -1),
    };

    let http_method = match method_str.to_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "PATCH" => Method::PATCH,
        "HEAD" => Method::HEAD,
        "OPTIONS" => Method::OPTIONS,
        _ => return FfiResult::error(format!("Unsupported HTTP method: {}", method_str), -1),
    };

    let body_opt = safe_cstr_to_string(body);
    let headers_opt = safe_cstr_to_string(headers);

    let client_ref = &(*client).0;

    match RUNTIME.block_on(async {
        let mut rb = client_ref.request(http_method, &url_str);

        // Apply headers if provided
        if let Some(headers_json) = headers_opt {
            if let Ok(headers_map) =
                serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&headers_json)
            {
                for (name, value) in headers_map {
                    if let Some(value_str) = value.as_str() {
                        rb = rb.header(name.as_str(), value_str);
                    }
                }
            }
        }

        // Apply body if provided
        if let Some(body_content) = body_opt {
            rb = rb.body(body_content);
        }

        let response = rb.send().await?;
        let text = response.text().await?;
        Ok::<_, pubky::Error>(text)
    }) {
        Ok(text) => FfiResult::success(text),
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// Perform an HTTP request and return the response as bytes.
///
/// # Arguments
/// * `client` - Pointer to the PubkyHttpClient
/// * `method` - HTTP method string (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
/// * `url` - The URL to request (can be pubky:// or https://)
/// * `body` - Optional request body as bytes (can be null)
/// * `body_len` - Length of the body bytes
/// * `headers` - Optional headers as JSON object string (can be null)
///
/// # Safety
/// The client pointer must be valid. Method and url must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn pubky_http_client_request_bytes(
    client: *const FfiHttpClient,
    method: *const c_char,
    url: *const c_char,
    body: *const u8,
    body_len: usize,
    headers: *const c_char,
) -> FfiBytesResult {
    if client.is_null() {
        return FfiBytesResult::error("Null client pointer".to_string(), -1);
    }

    let method_str = match safe_cstr_to_string(method) {
        Some(s) => s,
        None => return FfiBytesResult::error("Invalid or null method".to_string(), -1),
    };

    let url_str = match safe_cstr_to_string(url) {
        Some(s) => s,
        None => return FfiBytesResult::error("Invalid or null URL".to_string(), -1),
    };

    let http_method = match method_str.to_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "PATCH" => Method::PATCH,
        "HEAD" => Method::HEAD,
        "OPTIONS" => Method::OPTIONS,
        _ => return FfiBytesResult::error(format!("Unsupported HTTP method: {}", method_str), -1),
    };

    let body_opt = if body.is_null() || body_len == 0 {
        None
    } else {
        Some(std::slice::from_raw_parts(body, body_len).to_vec())
    };

    let headers_opt = safe_cstr_to_string(headers);

    let client_ref = &(*client).0;

    match RUNTIME.block_on(async {
        let mut rb = client_ref.request(http_method, &url_str);

        // Apply headers if provided
        if let Some(headers_json) = headers_opt {
            if let Ok(headers_map) =
                serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&headers_json)
            {
                for (name, value) in headers_map {
                    if let Some(value_str) = value.as_str() {
                        rb = rb.header(name.as_str(), value_str);
                    }
                }
            }
        }

        // Apply body if provided
        if let Some(body_content) = body_opt {
            rb = rb.body(body_content);
        }

        let response = rb.send().await?;
        let bytes = response.bytes().await?;
        Ok::<_, pubky::Error>(bytes.to_vec())
    }) {
        Ok(bytes) => FfiBytesResult::success(bytes),
        Err(e) => FfiBytesResult::error(e.to_string(), 1),
    }
}

/// HTTP response structure for detailed response information.
#[repr(C)]
pub struct FfiHttpResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response body as text (null if error or binary).
    pub body: *mut c_char,
    /// Response headers as JSON string.
    pub headers: *mut c_char,
    /// Error message (if error), null otherwise.
    pub error: *mut c_char,
    /// 0 for success, non-zero error code otherwise.
    pub code: i32,
}

impl FfiHttpResponse {
    /// Create a successful response.
    pub fn success(status: u16, body: String, headers: String) -> Self {
        let body_cstr =
            std::ffi::CString::new(body).unwrap_or_else(|_| std::ffi::CString::new("").unwrap());
        let headers_cstr = std::ffi::CString::new(headers)
            .unwrap_or_else(|_| std::ffi::CString::new("{}").unwrap());
        Self {
            status,
            body: body_cstr.into_raw(),
            headers: headers_cstr.into_raw(),
            error: ptr::null_mut(),
            code: 0,
        }
    }

    /// Create an error response.
    pub fn error(message: String, code: i32) -> Self {
        let error_cstr = std::ffi::CString::new(message)
            .unwrap_or_else(|_| std::ffi::CString::new("Unknown error").unwrap());
        Self {
            status: 0,
            body: ptr::null_mut(),
            headers: ptr::null_mut(),
            error: error_cstr.into_raw(),
            code,
        }
    }
}

/// Free an FfiHttpResponse structure.
///
/// # Safety
/// The response must have been returned by a pubky FFI function.
#[no_mangle]
pub unsafe extern "C" fn pubky_http_response_free(response: FfiHttpResponse) {
    if !response.body.is_null() {
        drop(std::ffi::CString::from_raw(response.body));
    }
    if !response.headers.is_null() {
        drop(std::ffi::CString::from_raw(response.headers));
    }
    if !response.error.is_null() {
        drop(std::ffi::CString::from_raw(response.error));
    }
}

/// Perform an HTTP request and return detailed response information.
///
/// Returns status code, body, and headers.
///
/// # Arguments
/// * `client` - Pointer to the PubkyHttpClient
/// * `method` - HTTP method string (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
/// * `url` - The URL to request (can be pubky:// or https://)
/// * `body` - Optional request body (can be null)
/// * `headers` - Optional headers as JSON object string (can be null)
///
/// # Safety
/// The client pointer must be valid. Method and url must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn pubky_http_client_request_full(
    client: *const FfiHttpClient,
    method: *const c_char,
    url: *const c_char,
    body: *const c_char,
    headers: *const c_char,
) -> FfiHttpResponse {
    if client.is_null() {
        return FfiHttpResponse::error("Null client pointer".to_string(), -1);
    }

    let method_str = match safe_cstr_to_string(method) {
        Some(s) => s,
        None => return FfiHttpResponse::error("Invalid or null method".to_string(), -1),
    };

    let url_str = match safe_cstr_to_string(url) {
        Some(s) => s,
        None => return FfiHttpResponse::error("Invalid or null URL".to_string(), -1),
    };

    let http_method = match method_str.to_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "PATCH" => Method::PATCH,
        "HEAD" => Method::HEAD,
        "OPTIONS" => Method::OPTIONS,
        _ => return FfiHttpResponse::error(format!("Unsupported HTTP method: {}", method_str), -1),
    };

    let body_opt = safe_cstr_to_string(body);
    let headers_opt = safe_cstr_to_string(headers);

    let client_ref = &(*client).0;

    match RUNTIME.block_on(async {
        let mut rb = client_ref.request(http_method, &url_str);

        // Apply headers if provided
        if let Some(headers_json) = headers_opt {
            if let Ok(headers_map) =
                serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&headers_json)
            {
                for (name, value) in headers_map {
                    if let Some(value_str) = value.as_str() {
                        rb = rb.header(name.as_str(), value_str);
                    }
                }
            }
        }

        // Apply body if provided
        if let Some(body_content) = body_opt {
            rb = rb.body(body_content);
        }

        let response = rb.send().await?;
        let status = response.status().as_u16();

        // Collect headers as JSON
        let mut headers_map = serde_json::Map::new();
        for (name, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                headers_map.insert(name.to_string(), serde_json::Value::String(v.to_string()));
            }
        }
        let headers_json = serde_json::to_string(&headers_map).unwrap_or_else(|_| "{}".to_string());

        let body_text = response.text().await?;
        Ok::<_, pubky::Error>((status, body_text, headers_json))
    }) {
        Ok((status, body, headers)) => FfiHttpResponse::success(status, body, headers),
        Err(e) => FfiHttpResponse::error(e.to_string(), 1),
    }
}
