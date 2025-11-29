//! FFI bindings for the Pubky facade.

use std::ptr;

use crate::error::FfiResult;
use crate::keypair::{FfiKeypair, FfiPublicKey};
use crate::runtime::RUNTIME;
use crate::signer::FfiSigner;
use crate::storage::FfiPublicStorage;

/// Opaque handle to a Pubky facade.
pub struct FfiPubky(pub(crate) pubky::Pubky);

/// Create a new Pubky facade with mainnet defaults.
/// Returns a pointer to the Pubky instance, or null on failure.
/// The caller must free the instance with `pubky_free`.
#[no_mangle]
pub extern "C" fn pubky_new() -> *mut FfiPubky {
    match pubky::Pubky::new() {
        Ok(pubky) => Box::into_raw(Box::new(FfiPubky(pubky))),
        Err(_) => ptr::null_mut(),
    }
}

/// Create a Pubky facade preconfigured for a local testnet.
/// Returns a pointer to the Pubky instance, or null on failure.
/// The caller must free the instance with `pubky_free`.
#[no_mangle]
pub extern "C" fn pubky_testnet() -> *mut FfiPubky {
    match pubky::Pubky::testnet() {
        Ok(pubky) => Box::into_raw(Box::new(FfiPubky(pubky))),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a Pubky instance.
///
/// # Safety
/// The pubky pointer must have been returned by a pubky FFI function.
#[no_mangle]
pub unsafe extern "C" fn pubky_free(pubky: *mut FfiPubky) {
    if !pubky.is_null() {
        drop(Box::from_raw(pubky));
    }
}

/// Create a signer from a Pubky instance and keypair.
/// Returns a pointer to the signer.
/// The caller must free the signer with `pubky_signer_free`.
///
/// # Safety
/// The pubky and keypair pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_signer(
    pubky: *const FfiPubky,
    keypair: *const FfiKeypair,
) -> *mut FfiSigner {
    if pubky.is_null() || keypair.is_null() {
        return ptr::null_mut();
    }

    let pubky = &(*pubky).0;
    let keypair = (*keypair).0.clone();
    let signer = pubky.signer(keypair);
    Box::into_raw(Box::new(FfiSigner(signer)))
}

/// Get a public storage handle from a Pubky instance.
/// Returns a pointer to the public storage.
/// The caller must free the storage with `pubky_public_storage_free`.
///
/// # Safety
/// The pubky pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_public_storage(pubky: *const FfiPubky) -> *mut FfiPublicStorage {
    if pubky.is_null() {
        return ptr::null_mut();
    }

    let pubky = &(*pubky).0;
    Box::into_raw(Box::new(FfiPublicStorage(pubky.public_storage())))
}

/// Resolve the homeserver for a given public key.
/// Returns a result containing the homeserver public key z32 string, or null if not found.
///
/// # Safety
/// The pubky and user_public_key pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn pubky_get_homeserver_of(
    pubky: *const FfiPubky,
    user_public_key: *const FfiPublicKey,
) -> FfiResult {
    if pubky.is_null() || user_public_key.is_null() {
        return FfiResult::error("Null pointer".to_string(), -1);
    }

    let pubky = &(*pubky).0;
    let user_pk = &(*user_public_key).0;

    match RUNTIME.block_on(pubky.get_homeserver_of(user_pk)) {
        Some(pk) => FfiResult::success(pk.to_string()),
        None => FfiResult::success_empty(),
    }
}
