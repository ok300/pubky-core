//! Global async runtime and Pubky instance for FFI.
//!
//! This module provides a single, global, multi-threaded Tokio runtime
//! and a global Pubky instance. All FFI operations use this runtime
//! via `block_on` to execute async operations synchronously.
//!
//! This design avoids issues with creating/destroying runtimes per-call
//! and provides efficient connection pooling.

use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

/// Global Tokio runtime for executing async operations from FFI.
/// Built on first use with multi-threaded execution.
pub(crate) static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("pubky-ffi-tokio")
        .build()
        .expect("Failed to build Tokio runtime")
});

/// Global Pubky instance for mainnet operations.
/// Connection pooling is handled automatically by the underlying HTTP client.
pub(crate) static GLOBAL_PUBKY: Lazy<pubky::Pubky> = Lazy::new(|| {
    pubky::Pubky::new().expect("Failed to create global Pubky instance")
});

/// Global Pubky instance for testnet operations.
/// Only initialized if testnet operations are used.
pub(crate) static GLOBAL_PUBKY_TESTNET: Lazy<pubky::Pubky> = Lazy::new(|| {
    pubky::Pubky::testnet().expect("Failed to create global Pubky testnet instance")
});

/// Initialize the FFI runtime. Should be called once at application startup.
/// Returns 0 on success, non-zero on failure.
///
/// This initializes the global Tokio runtime and Pubky instance.
#[no_mangle]
pub extern "C" fn pubky_init() -> i32 {
    // Force initialization of the runtime and global pubky instance
    let _ = &*RUNTIME;
    let _ = &*GLOBAL_PUBKY;
    0
}

/// Initialize the FFI runtime for testnet. Should be called once at application startup.
/// Returns 0 on success, non-zero on failure.
///
/// This initializes the global Tokio runtime and testnet Pubky instance.
#[no_mangle]
pub extern "C" fn pubky_init_testnet() -> i32 {
    // Force initialization of the runtime and global testnet pubky instance
    let _ = &*RUNTIME;
    let _ = &*GLOBAL_PUBKY_TESTNET;
    0
}
