//! FFI bindings for the PubkySession actor.

use std::ptr;

use crate::error::FfiResult;
use crate::keypair::FfiPublicKey;
use crate::runtime::block_on;
use crate::storage::FfiSessionStorage;

/// Opaque handle to a PubkySession.
pub struct FfiSession(pub(crate) pubky::PubkySession);

/// Get the public key of the session user.
/// Returns a pointer to the public key.
/// The caller must free the public key with `pubky_public_key_free`.
///
/// # Safety
/// The session pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_public_key(session: *const FfiSession) -> *mut FfiPublicKey {
    if session.is_null() {
        return ptr::null_mut();
    }

    let session = &(*session).0;
    Box::into_raw(Box::new(FfiPublicKey(session.info().public_key().clone())))
}

/// Get the capabilities string of the session.
/// Returns a result containing the capabilities string.
///
/// # Safety
/// The session pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_capabilities(session: *const FfiSession) -> FfiResult {
    if session.is_null() {
        return FfiResult::error("Null session pointer".to_string(), -1);
    }

    let session = &(*session).0;
    let caps = session.info().capabilities();
    let caps_str: Vec<String> = caps.iter().map(|c| c.to_string()).collect();
    FfiResult::success(caps_str.join(","))
}

/// Get a session storage handle from a session.
/// Returns a pointer to the session storage.
/// The caller must free the storage with `pubky_session_storage_free`.
///
/// # Safety
/// The session pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_storage(
    session: *const FfiSession,
) -> *mut FfiSessionStorage {
    if session.is_null() {
        return ptr::null_mut();
    }

    let session = &(*session).0;
    Box::into_raw(Box::new(FfiSessionStorage(session.storage())))
}

/// Sign out and invalidate the session.
/// Returns an FfiResult with success/error status.
/// Note: This consumes the session. After calling, the session pointer is invalid.
///
/// # Safety
/// The session pointer must be valid and owned (will be consumed).
#[no_mangle]
pub unsafe extern "C" fn pubky_session_signout(session: *mut FfiSession) -> FfiResult {
    if session.is_null() {
        return FfiResult::error("Null session pointer".to_string(), -1);
    }

    // Take ownership of the session
    let session = Box::from_raw(session);
    let inner_session = session.0;

    match block_on(inner_session.signout()) {
        Ok(()) => FfiResult::success_empty(),
        Err((e, _)) => FfiResult::from_pubky_error(e),
    }
}

/// Revalidate the session with the homeserver.
/// Returns an FfiResult with "valid", "invalid", or an error.
///
/// # Safety
/// The session pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_revalidate(session: *const FfiSession) -> FfiResult {
    if session.is_null() {
        return FfiResult::error("Null session pointer".to_string(), -1);
    }

    let session = &(*session).0;

    match block_on(session.revalidate()) {
        Ok(Some(_)) => FfiResult::success("valid".to_string()),
        Ok(None) => FfiResult::success("invalid".to_string()),
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// Free a session.
///
/// # Safety
/// The session pointer must have been returned by a pubky FFI function.
#[no_mangle]
pub unsafe extern "C" fn pubky_session_free(session: *mut FfiSession) {
    if !session.is_null() {
        drop(Box::from_raw(session));
    }
}
