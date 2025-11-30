use libc::c_char;
use once_cell::sync::Lazy;
use std::ffi::{CStr, CString};
use tokio::runtime::Runtime;
use pubky::{Keypair, Pubky, PubkySigner, PubkySession, PublicKey, Method, PubkyHttpClient};

// Singleton for the Tokio runtime
static RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());

// Singleton for the Pubky facade
static PUBKY: Lazy<Pubky> = Lazy::new(|| Pubky::new().unwrap());

// Singleton for the HttpClient
static HTTP_CLIENT: Lazy<PubkyHttpClient> = Lazy::new(|| PubkyHttpClient::new().unwrap());

/// Frees a string that was allocated by Rust.
#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(s);
    }
}

/// Generates a new random keypair.
#[no_mangle]
pub extern "C" fn keypair_random() -> *mut Keypair {
    let keypair = Keypair::random();
    Box::into_raw(Box::new(keypair))
}

/// Frees a keypair.
#[no_mangle]
pub extern "C" fn keypair_free(keypair: *mut Keypair) {
    if !keypair.is_null() {
        unsafe {
            let _ = Box::from_raw(keypair);
        }
    }
}

/// Creates a signer from a keypair.
#[no_mangle]
pub extern "C" fn signer_from_keypair(keypair: *const Keypair) -> *mut PubkySigner {
    let keypair = unsafe { &*keypair };
    let signer = PUBKY.signer(keypair.clone());
    Box::into_raw(Box::new(signer))
}

/// Frees a signer.
#[no_mangle]
pub extern "C" fn signer_free(signer: *mut PubkySigner) {
    if !signer.is_null() {
        unsafe {
            let _ = Box::from_raw(signer);
        }
    }
}


/// Signs up a user with a given homeserver.
#[no_mangle]
pub extern "C" fn signer_signup(signer: *mut PubkySigner, homeserver_pk: *const c_char) -> *mut PubkySession {
    let signer = unsafe { &*signer };
    let homeserver_pk = unsafe { CStr::from_ptr(homeserver_pk).to_str() };
    if homeserver_pk.is_err() {
        return std::ptr::null_mut();
    }
    let homeserver = PublicKey::try_from(homeserver_pk.unwrap());
    if homeserver.is_err() {
        return std::ptr::null_mut();
    }

    let session = RUNTIME.block_on(async {
        signer.signup(&homeserver.unwrap(), None).await
    });

    match session {
        Ok(session) => Box::into_raw(Box::new(session)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a session.
#[no_mangle]
pub extern "C" fn session_free(session: *mut PubkySession) {
    if !session.is_null() {
        unsafe {
            let _ = Box::from_raw(session);
        }
    }
}

/// Puts a value into storage for the given session.
#[no_mangle]
pub extern "C" fn session_storage_put(session: *mut PubkySession, path: *const c_char, value: *const c_char) -> i32 {
    let session = unsafe { &*session };
    let path = match unsafe { CStr::from_ptr(path).to_str() } {
        Ok(p) => p,
        Err(_) => return -1,
    };
    let value = match unsafe { CStr::from_ptr(value).to_str() } {
        Ok(v) => v,
        Err(_) => return -1,
    };

    let result = RUNTIME.block_on(async {
        session.storage().put(path, value).await
    });

    match result {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Gets a value from storage for the given session.
#[no_mangle]
pub extern "C" fn session_storage_get(session: *mut PubkySession, path: *const c_char) -> *mut c_char {
    let session = unsafe { &*session };
    let path = match unsafe { CStr::from_ptr(path).to_str() } {
        Ok(p) => p,
        Err(_) => return std::ptr::null_mut(),
    };

    let value = RUNTIME.block_on(async {
        session.storage().get(path).await
    });

    match value {
        Ok(response) => {
            let text = RUNTIME.block_on(async { response.text().await });
            match text {
                Ok(text) => match CString::new(text) {
                    Ok(c_string) => c_string.into_raw(),
                    Err(_) => std::ptr::null_mut(),
                },
                Err(_) => std::ptr::null_mut(),
            }
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Makes an HTTP request using the PubkyHttpClient.
#[no_mangle]
pub extern "C" fn http_request(method: *const c_char, url: *const c_char) -> *mut c_char {
    let method_str = match unsafe { CStr::from_ptr(method).to_str() } {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let url_str = match unsafe { CStr::from_ptr(url).to_str() } {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let method = match method_str.to_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "PATCH" => Method::PATCH,
        "HEAD" => Method::HEAD,
        "OPTIONS" => Method::OPTIONS,
        _ => Method::GET,
    };

    let url = match url::Url::parse(url_str) {
        Ok(u) => u,
        Err(_) => return std::ptr::null_mut(),
    };

    let response = RUNTIME.block_on(async {
        HTTP_CLIENT.request(method, &url).send().await
    });

    match response {
        Ok(response) => {
            let text = RUNTIME.block_on(async { response.text().await });
            match text {
                Ok(text) => match CString::new(text) {
                    Ok(c_string) => c_string.into_raw(),
                    Err(_) => std::ptr::null_mut(),
                },
                Err(_) => std::ptr::null_mut(),
            }
        },
        Err(_) => std::ptr::null_mut(),
    }
}
