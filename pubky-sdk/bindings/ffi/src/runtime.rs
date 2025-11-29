//! Global async runtime for FFI.
//!
//! Since FFI calls are synchronous, we need a global Tokio runtime
//! to execute async operations.

use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

/// Global Tokio runtime for executing async operations from FFI.
pub(crate) static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime")
});

/// Initialize the FFI runtime. Should be called once at application startup.
/// Returns 0 on success, non-zero on failure.
#[no_mangle]
pub extern "C" fn pubky_init() -> i32 {
    // Force initialization of the runtime
    let _ = &*RUNTIME;
    0
}
