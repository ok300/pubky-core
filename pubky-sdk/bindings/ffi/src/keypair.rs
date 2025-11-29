//! FFI bindings for Keypair and PublicKey operations.

use std::os::raw::c_char;
use std::ptr;

use pkarr::{Keypair, PublicKey};

use crate::error::{FfiBytesResult, FfiResult, cstr_to_string};

/// Opaque handle to a Keypair.
pub struct FfiKeypair(pub(crate) Keypair);

/// Opaque handle to a PublicKey.
pub struct FfiPublicKey(pub(crate) PublicKey);

/// Generate a random keypair.
/// Returns a pointer to the keypair, or null on failure.
/// The caller must free the keypair with `pubky_keypair_free`.
#[no_mangle]
pub extern "C" fn pubky_keypair_random() -> *mut FfiKeypair {
    Box::into_raw(Box::new(FfiKeypair(Keypair::random())))
}

/// Create a keypair from a 32-byte secret key.
/// Returns a pointer to the keypair, or null on failure.
/// The caller must free the keypair with `pubky_keypair_free`.
///
/// # Safety
/// The secret_key pointer must point to exactly 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn pubky_keypair_from_secret_key(
    secret_key: *const u8,
    secret_key_len: usize,
) -> *mut FfiKeypair {
    if secret_key.is_null() || secret_key_len != 32 {
        return ptr::null_mut();
    }

    let secret_slice = std::slice::from_raw_parts(secret_key, secret_key_len);
    let secret_array: [u8; 32] = match secret_slice.try_into() {
        Ok(arr) => arr,
        Err(_) => return ptr::null_mut(),
    };

    Box::into_raw(Box::new(FfiKeypair(Keypair::from_secret_key(&secret_array))))
}

/// Get the secret key from a keypair.
/// Returns a result containing the 32-byte secret key.
///
/// # Safety
/// The keypair pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_keypair_secret_key(keypair: *const FfiKeypair) -> FfiBytesResult {
    if keypair.is_null() {
        return FfiBytesResult::error("Null keypair pointer".to_string(), -1);
    }

    let keypair = &(*keypair).0;
    FfiBytesResult::success(keypair.secret_key().to_vec())
}

/// Get the public key from a keypair.
/// Returns a pointer to the public key.
/// The caller must free the public key with `pubky_public_key_free`.
///
/// # Safety
/// The keypair pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_keypair_public_key(keypair: *const FfiKeypair) -> *mut FfiPublicKey {
    if keypair.is_null() {
        return ptr::null_mut();
    }

    let keypair = &(*keypair).0;
    Box::into_raw(Box::new(FfiPublicKey(keypair.public_key())))
}

/// Free a keypair.
///
/// # Safety
/// The keypair pointer must have been returned by a pubky FFI function.
#[no_mangle]
pub unsafe extern "C" fn pubky_keypair_free(keypair: *mut FfiKeypair) {
    if !keypair.is_null() {
        drop(Box::from_raw(keypair));
    }
}

/// Create a recovery file for a keypair (encrypted with the given passphrase).
/// Returns a result containing the encrypted recovery file bytes.
///
/// # Safety
/// The keypair and passphrase pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_keypair_create_recovery_file(
    keypair: *const FfiKeypair,
    passphrase: *const c_char,
) -> FfiBytesResult {
    if keypair.is_null() {
        return FfiBytesResult::error("Null keypair pointer".to_string(), -1);
    }

    let passphrase = match cstr_to_string(passphrase) {
        Some(s) => s,
        None => return FfiBytesResult::error("Invalid passphrase".to_string(), -1),
    };

    let keypair = &(*keypair).0;
    let recovery_file = pubky_common::recovery_file::create_recovery_file(keypair, &passphrase);
    FfiBytesResult::success(recovery_file)
}

/// Decrypt a recovery file and return a keypair.
/// Returns a pointer to the keypair, or null on failure.
///
/// # Safety
/// The recovery_file and passphrase pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_keypair_from_recovery_file(
    recovery_file: *const u8,
    recovery_file_len: usize,
    passphrase: *const c_char,
) -> *mut FfiKeypair {
    if recovery_file.is_null() {
        return ptr::null_mut();
    }

    let passphrase = match cstr_to_string(passphrase) {
        Some(s) => s,
        None => return ptr::null_mut(),
    };

    let recovery_slice = std::slice::from_raw_parts(recovery_file, recovery_file_len);
    match pubky_common::recovery_file::decrypt_recovery_file(recovery_slice, &passphrase) {
        Ok(keypair) => Box::into_raw(Box::new(FfiKeypair(keypair))),
        Err(_) => ptr::null_mut(),
    }
}

// --- PublicKey operations ---

/// Get the z-base32 encoded string representation of a public key.
/// Returns a result containing the z32 string.
///
/// # Safety
/// The public_key pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_public_key_z32(public_key: *const FfiPublicKey) -> FfiResult {
    if public_key.is_null() {
        return FfiResult::error("Null public key pointer".to_string(), -1);
    }

    let pk = &(*public_key).0;
    FfiResult::success(pk.to_string())
}

/// Get the raw bytes of a public key.
/// Returns a result containing the 32-byte public key.
///
/// # Safety
/// The public_key pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_public_key_bytes(public_key: *const FfiPublicKey) -> FfiBytesResult {
    if public_key.is_null() {
        return FfiBytesResult::error("Null public key pointer".to_string(), -1);
    }

    let pk = &(*public_key).0;
    FfiBytesResult::success(pk.as_bytes().to_vec())
}

/// Create a public key from a z-base32 encoded string.
/// Returns a pointer to the public key, or null on failure.
///
/// # Safety
/// The z32 pointer must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pubky_public_key_from_z32(z32: *const c_char) -> *mut FfiPublicKey {
    let z32_str = match cstr_to_string(z32) {
        Some(s) => s,
        None => return ptr::null_mut(),
    };

    match PublicKey::try_from(z32_str.as_str()) {
        Ok(pk) => Box::into_raw(Box::new(FfiPublicKey(pk))),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a public key.
///
/// # Safety
/// The public_key pointer must have been returned by a pubky FFI function.
#[no_mangle]
pub unsafe extern "C" fn pubky_public_key_free(public_key: *mut FfiPublicKey) {
    if !public_key.is_null() {
        drop(Box::from_raw(public_key));
    }
}
