//! FFI bindings for the Pubky facade.
//!
//! Uses lazy-initialized global Pubky instances for mainnet and testnet.
//! All operations use `block_on()` to execute async calls synchronously.

use std::ptr;

use crate::error::FfiResult;
use crate::keypair::{FfiKeypair, FfiPublicKey};
use crate::runtime::{block_on, get_global_pubky, get_global_pubky_testnet};
use crate::signer::FfiSigner;
use crate::storage::FfiPublicStorage;

/// Opaque handle to a Pubky facade.
/// This is a thin wrapper that indicates whether to use mainnet or testnet.
pub struct FfiPubky {
    /// Whether this uses the testnet global instance.
    pub(crate) testnet: bool,
}

impl FfiPubky {
    /// Get a reference to the appropriate global Pubky instance.
    pub(crate) fn get(&self) -> &'static pubky::Pubky {
        if self.testnet {
            get_global_pubky_testnet()
        } else {
            get_global_pubky()
        }
    }
}

/// Create a new Pubky facade with mainnet defaults.
/// Returns a pointer to the Pubky instance, or null on failure.
/// The caller must free the instance with `pubky_free`.
///
/// This uses a global, lazy-initialized Pubky instance with connection pooling.
#[no_mangle]
pub extern "C" fn pubky_new() -> *mut FfiPubky {
    // Force initialization of global mainnet instance
    let _ = get_global_pubky();
    Box::into_raw(Box::new(FfiPubky { testnet: false }))
}

/// Create a Pubky facade preconfigured for a local testnet.
/// Returns a pointer to the Pubky instance, or null on failure.
/// The caller must free the instance with `pubky_free`.
///
/// This uses a global, lazy-initialized testnet Pubky instance with connection pooling.
#[no_mangle]
pub extern "C" fn pubky_testnet() -> *mut FfiPubky {
    // Force initialization of global testnet instance
    let _ = get_global_pubky_testnet();
    Box::into_raw(Box::new(FfiPubky { testnet: true }))
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

    let pubky_handle = &*pubky;
    let keypair = (*keypair).0.clone();
    let signer = pubky_handle.get().signer(keypair);
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

    let pubky_handle = &*pubky;
    Box::into_raw(Box::new(FfiPublicStorage(pubky_handle.get().public_storage())))
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

    let pubky_handle = &*pubky;
    let user_pk = &(*user_public_key).0;

    match block_on(pubky_handle.get().get_homeserver_of(user_pk)) {
        Some(pk) => FfiResult::success(pk.to_string()),
        None => FfiResult::success_empty(),
    }
}
