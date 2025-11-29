//! FFI bindings for the PubkySigner actor.

use std::os::raw::c_char;
use std::ptr;

use crate::error::{cstr_to_string, FfiResult};
use crate::keypair::FfiPublicKey;
use crate::runtime::RUNTIME;
use crate::session::FfiSession;

/// Opaque handle to a PubkySigner.
pub struct FfiSigner(pub(crate) pubky::PubkySigner);

/// Get the public key of a signer.
/// Returns a pointer to the public key.
/// The caller must free the public key with `pubky_public_key_free`.
///
/// # Safety
/// The signer pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_signer_public_key(signer: *const FfiSigner) -> *mut FfiPublicKey {
    if signer.is_null() {
        return ptr::null_mut();
    }

    let signer = &(*signer).0;
    Box::into_raw(Box::new(FfiPublicKey(signer.public_key())))
}

/// Sign up at a homeserver.
/// Returns a pointer to the session on success.
/// The caller must free the session with `pubky_session_free`.
///
/// # Safety
/// The signer and homeserver pointers must be valid.
/// signup_token can be null for no token.
#[no_mangle]
pub unsafe extern "C" fn pubky_signer_signup(
    signer: *const FfiSigner,
    homeserver: *const FfiPublicKey,
    signup_token: *const c_char,
) -> *mut FfiSession {
    if signer.is_null() || homeserver.is_null() {
        return ptr::null_mut();
    }

    let signer = &(*signer).0;
    let homeserver_pk = &(*homeserver).0;
    let token = cstr_to_string(signup_token);

    match RUNTIME.block_on(signer.signup(homeserver_pk, token.as_deref())) {
        Ok(session) => Box::into_raw(Box::new(FfiSession(session))),
        Err(_) => ptr::null_mut(),
    }
}

/// Sign up at a homeserver with detailed error information.
/// Returns an FfiResult containing the session pointer as a hex string on success,
/// or an error message on failure.
///
/// # Safety
/// The signer and homeserver pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_signer_signup_with_result(
    signer: *const FfiSigner,
    homeserver: *const FfiPublicKey,
    signup_token: *const c_char,
    session_out: *mut *mut FfiSession,
) -> FfiResult {
    if signer.is_null() || homeserver.is_null() || session_out.is_null() {
        return FfiResult::error("Null pointer".to_string(), -1);
    }

    let signer = &(*signer).0;
    let homeserver_pk = &(*homeserver).0;
    let token = cstr_to_string(signup_token);

    match RUNTIME.block_on(signer.signup(homeserver_pk, token.as_deref())) {
        Ok(session) => {
            *session_out = Box::into_raw(Box::new(FfiSession(session)));
            FfiResult::success_empty()
        }
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// Sign in (for returning users).
/// Returns a pointer to the session on success.
/// The caller must free the session with `pubky_session_free`.
///
/// # Safety
/// The signer pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_signer_signin(signer: *const FfiSigner) -> *mut FfiSession {
    if signer.is_null() {
        return ptr::null_mut();
    }

    let signer = &(*signer).0;

    match RUNTIME.block_on(signer.signin()) {
        Ok(session) => Box::into_raw(Box::new(FfiSession(session))),
        Err(_) => ptr::null_mut(),
    }
}

/// Sign in with detailed error information.
/// Returns an FfiResult with error details.
///
/// # Safety
/// The signer and session_out pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_signer_signin_with_result(
    signer: *const FfiSigner,
    session_out: *mut *mut FfiSession,
) -> FfiResult {
    if signer.is_null() || session_out.is_null() {
        return FfiResult::error("Null pointer".to_string(), -1);
    }

    let signer = &(*signer).0;

    match RUNTIME.block_on(signer.signin()) {
        Ok(session) => {
            *session_out = Box::into_raw(Box::new(FfiSession(session)));
            FfiResult::success_empty()
        }
        Err(e) => FfiResult::from_pubky_error(e),
    }
}

/// Sign in blocking (waits for PKDNS publish to complete).
/// Returns a pointer to the session on success.
/// The caller must free the session with `pubky_session_free`.
///
/// # Safety
/// The signer pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_signer_signin_blocking(signer: *const FfiSigner) -> *mut FfiSession {
    if signer.is_null() {
        return ptr::null_mut();
    }

    let signer = &(*signer).0;

    match RUNTIME.block_on(signer.signin_blocking()) {
        Ok(session) => Box::into_raw(Box::new(FfiSession(session))),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a signer.
///
/// # Safety
/// The signer pointer must have been returned by a pubky FFI function.
#[no_mangle]
pub unsafe extern "C" fn pubky_signer_free(signer: *mut FfiSigner) {
    if !signer.is_null() {
        drop(Box::from_raw(signer));
    }
}
