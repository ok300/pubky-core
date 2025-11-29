# Pubky SDK FFI Bindings

C-compatible FFI bindings for the Pubky SDK, enabling integration with languages that support C FFI (Ruby, Python, Go, etc.).

## Architecture

The FFI bindings use:
- A **global Tokio multi-threaded runtime** for async SDK operations (keypair, signer, session, storage)
- A **global blocking reqwest HTTP client** for HTTP requests (simpler, avoids async runtime conflicts)

This hybrid approach provides the best of both worlds:
- Full SDK functionality through the async runtime
- Simple, reliable HTTP requests through the blocking client

## Building

```bash
# Build the shared library (release)
cargo build --release --package pubky-sdk-ffi

# The library will be at:
# - Linux: target/release/libpubky_sdk_ffi.so
# - macOS: target/release/libpubky_sdk_ffi.dylib
# - Windows: target/release/pubky_sdk_ffi.dll
```

## C Header

The FFI exposes the following main functions. Here's a summary of the C API:

```c
// Initialization
int pubky_init(void);

// Keypair operations
void* pubky_keypair_random(void);
void* pubky_keypair_from_secret_key(const uint8_t* secret_key, size_t len);
FfiBytesResult pubky_keypair_secret_key(const void* keypair);
void* pubky_keypair_public_key(const void* keypair);
void pubky_keypair_free(void* keypair);

// Recovery file operations
FfiBytesResult pubky_keypair_create_recovery_file(const void* keypair, const char* passphrase);
void* pubky_keypair_from_recovery_file(const uint8_t* data, size_t len, const char* passphrase);

// PublicKey operations
FfiResult pubky_public_key_z32(const void* public_key);
FfiBytesResult pubky_public_key_bytes(const void* public_key);
void* pubky_public_key_from_z32(const char* z32);
void pubky_public_key_free(void* public_key);

// Pubky facade
void* pubky_new(void);
void* pubky_testnet(void);
void pubky_free(void* pubky);
void* pubky_signer(const void* pubky, const void* keypair);
void* pubky_public_storage(const void* pubky);
FfiResult pubky_get_homeserver_of(const void* pubky, const void* public_key);

// HTTP Client (low-level request API)
void* pubky_http_client_new(void);
void* pubky_http_client_testnet(void);
void pubky_http_client_free(void* client);
FfiResult pubky_http_client_request(const void* client, const char* method, const char* url, const char* body, const char* headers);
FfiBytesResult pubky_http_client_request_bytes(const void* client, const char* method, const char* url, const uint8_t* body, size_t body_len, const char* headers);
FfiHttpResponse pubky_http_client_request_full(const void* client, const char* method, const char* url, const char* body, const char* headers);
void pubky_http_response_free(FfiHttpResponse response);

// Signer operations
void* pubky_signer_public_key(const void* signer);
void* pubky_signer_signup(const void* signer, const void* homeserver, const char* token);
FfiResult pubky_signer_signup_with_result(const void* signer, const void* homeserver, const char* token, void** session_out);
void* pubky_signer_signin(const void* signer);
FfiResult pubky_signer_signin_with_result(const void* signer, void** session_out);
void* pubky_signer_signin_blocking(const void* signer);
void pubky_signer_free(void* signer);

// Session operations
void* pubky_session_public_key(const void* session);
FfiResult pubky_session_capabilities(const void* session);
void* pubky_session_storage(const void* session);
FfiResult pubky_session_signout(void* session);
FfiResult pubky_session_revalidate(const void* session);
void pubky_session_free(void* session);

// Session storage operations
FfiResult pubky_session_storage_get_text(const void* storage, const char* path);
FfiBytesResult pubky_session_storage_get_bytes(const void* storage, const char* path);
FfiResult pubky_session_storage_get_json(const void* storage, const char* path);
FfiResult pubky_session_storage_put_text(const void* storage, const char* path, const char* body);
FfiResult pubky_session_storage_put_bytes(const void* storage, const char* path, const uint8_t* body, size_t len);
FfiResult pubky_session_storage_put_json(const void* storage, const char* path, const char* json);
FfiResult pubky_session_storage_delete(const void* storage, const char* path);
FfiResult pubky_session_storage_exists(const void* storage, const char* path);
FfiResult pubky_session_storage_list(const void* storage, const char* path, uint16_t limit);
void pubky_session_storage_free(void* storage);

// Public storage operations
FfiResult pubky_public_storage_get_text(const void* storage, const char* address);
FfiBytesResult pubky_public_storage_get_bytes(const void* storage, const char* address);
FfiResult pubky_public_storage_get_json(const void* storage, const char* address);
FfiResult pubky_public_storage_exists(const void* storage, const char* address);
FfiResult pubky_public_storage_list(const void* storage, const char* address, uint16_t limit);
void pubky_public_storage_free(void* storage);

// Memory management
void pubky_string_free(char* ptr);
void pubky_bytes_free(uint8_t* ptr, size_t len);
void pubky_result_free(FfiResult result);
void pubky_bytes_result_free(FfiBytesResult result);
```

## Result Types

```c
typedef struct {
    char* data;     // Result data (if success), null otherwise
    char* error;    // Error message (if error), null otherwise
    int32_t code;   // 0 for success, non-zero error code otherwise
} FfiResult;

typedef struct {
    uint8_t* data;  // Result data (if success), null otherwise
    size_t len;     // Length of data
    char* error;    // Error message (if error), null otherwise
    int32_t code;   // 0 for success, non-zero error code otherwise
} FfiBytesResult;

typedef struct {
    uint16_t status;  // HTTP status code
    char* body;       // Response body as text
    char* headers;    // Response headers as JSON string
    char* error;      // Error message (if error), null otherwise
    int32_t code;     // 0 for success, non-zero error code otherwise
} FfiHttpResponse;
```

## Error Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Request error (HTTP transport/server) |
| 2 | Pkarr error (DHT operation) |
| 3 | Parse error (URL parsing) |
| 4 | Authentication error |
| 5 | Build error (client construction) |
| -1 | Invalid input/null pointer |

## Usage Example (C)

```c
#include <stdio.h>
#include <stdint.h>
#include <string.h>

// Declare FFI functions (or include generated header)
extern int pubky_init(void);
extern void* pubky_keypair_random(void);
extern void* pubky_new(void);
// ... other declarations

int main() {
    // Initialize the runtime
    pubky_init();
    
    // Create a Pubky facade
    void* pubky = pubky_new();
    if (!pubky) {
        fprintf(stderr, "Failed to create Pubky instance\n");
        return 1;
    }
    
    // Generate a random keypair
    void* keypair = pubky_keypair_random();
    
    // Create a signer
    void* signer = pubky_signer(pubky, keypair);
    
    // Get public key
    void* public_key = pubky_keypair_public_key(keypair);
    FfiResult z32_result = pubky_public_key_z32(public_key);
    if (z32_result.code == 0) {
        printf("Public key: %s\n", z32_result.data);
        pubky_string_free(z32_result.data);
    }
    
    // Clean up
    pubky_public_key_free(public_key);
    pubky_signer_free(signer);
    pubky_keypair_free(keypair);
    pubky_free(pubky);
    
    return 0;
}
```

## Low-Level HTTP Client Example (C)

```c
#include <stdio.h>
#include <stdint.h>

extern int pubky_init(void);
extern void* pubky_http_client_new(void);
extern FfiHttpResponse pubky_http_client_request_full(const void*, const char*, const char*, const char*, const char*);
extern void pubky_http_response_free(FfiHttpResponse);
extern void pubky_http_client_free(void*);

int main() {
    pubky_init();
    
    // Create HTTP client
    void* client = pubky_http_client_new();
    
    // Make a GET request to any URL (HTTPS, pubky://, or _pubky.*)
    FfiHttpResponse resp = pubky_http_client_request_full(
        client,
        "GET",
        "https://example.com",
        NULL,  // no body
        NULL   // no custom headers
    );
    
    if (resp.code == 0) {
        printf("Status: %d\n", resp.status);
        printf("Body: %s\n", resp.body);
    } else {
        printf("Error: %s\n", resp.error);
    }
    
    pubky_http_response_free(resp);
    pubky_http_client_free(client);
    return 0;
}
```

## See Also

- [Ruby on Rails Integration Guide](./RAILS_INTEGRATION.md) - Detailed guide for using Pubky SDK in Ruby on Rails applications
- [Pubky SDK README](../../README.md) - Main SDK documentation

---

**License:** MIT
