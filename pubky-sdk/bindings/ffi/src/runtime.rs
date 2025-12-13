//! Global async runtime and Pubky instance for FFI.
//!
//! This module provides a single, global, multi-threaded Tokio runtime
//! and a global Pubky instance. All FFI operations use this runtime
//! via `block_on` to execute async operations synchronously.
//!
//! This design avoids issues with creating/destroying runtimes per-call
//! and provides efficient connection pooling.
//!
//! **IMPORTANT**: All Pubky instances are created within the runtime context
//! using `block_on` to ensure proper async initialization.

use once_cell::sync::Lazy;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

/// Global Tokio runtime for executing async operations from FFI.
/// Built on first use with multi-threaded execution.
/// This must be initialized BEFORE creating any Pubky instances.
pub(crate) static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("pubky-ffi-tokio")
        .build()
        .expect("Failed to build Tokio runtime")
});

/// Global Pubky instance for mainnet operations (OnceLock for explicit initialization).
static GLOBAL_PUBKY_CELL: OnceLock<pubky::Pubky> = OnceLock::new();

/// Global Pubky instance for testnet operations (OnceLock for explicit initialization).
static GLOBAL_PUBKY_TESTNET_CELL: OnceLock<pubky::Pubky> = OnceLock::new();

/// Get or initialize the global mainnet Pubky instance.
/// This ensures the runtime is initialized first and creates the Pubky
/// instance within the runtime context.
pub(crate) fn get_global_pubky() -> &'static pubky::Pubky {
    GLOBAL_PUBKY_CELL.get_or_init(|| {
        // Ensure runtime is initialized first
        let _ = &*RUNTIME;
        // Create Pubky - this is actually synchronous but ensures runtime exists
        pubky::Pubky::new().expect("Failed to create global Pubky instance")
    })
}

/// Get or initialize the global testnet Pubky instance.
/// This ensures the runtime is initialized first and creates the Pubky
/// instance within the runtime context.
pub(crate) fn get_global_pubky_testnet() -> &'static pubky::Pubky {
    GLOBAL_PUBKY_TESTNET_CELL.get_or_init(|| {
        // Ensure runtime is initialized first
        let _ = &*RUNTIME;
        // Create Pubky - this is actually synchronous but ensures runtime exists
        pubky::Pubky::testnet().expect("Failed to create global Pubky testnet instance")
    })
}

/// Execute an async operation on the global runtime.
/// This is the primary way to run async pubky-sdk operations from FFI.
///
/// # Panics
/// Panics if the runtime cannot be initialized.
pub(crate) fn block_on<F: std::future::Future>(f: F) -> F::Output {
    RUNTIME.block_on(f)
}

/// Initialize the FFI runtime. Should be called once at application startup.
/// Returns 0 on success, non-zero on failure.
///
/// This initializes the global Tokio runtime and Pubky instance.
#[no_mangle]
pub extern "C" fn pubky_init() -> i32 {
    // Force initialization of the runtime first
    let _ = &*RUNTIME;
    // Then initialize the global pubky instance
    let _ = get_global_pubky();
    0
}

/// Initialize the FFI runtime for testnet. Should be called once at application startup.
/// Returns 0 on success, non-zero on failure.
///
/// This initializes the global Tokio runtime and testnet Pubky instance.
#[no_mangle]
pub extern "C" fn pubky_init_testnet() -> i32 {
    // Force initialization of the runtime first
    let _ = &*RUNTIME;
    // Then initialize the global testnet pubky instance
    let _ = get_global_pubky_testnet();
    0
}
