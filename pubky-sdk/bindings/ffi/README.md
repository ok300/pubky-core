# Pubky SDK FFI

FFI (Foreign Function Interface) bindings for the Pubky SDK, enabling integration with other languages like Ruby, Python, and more.

## Features

- **Singleton pattern**: The `HttpClient` and `Pubky` facade are lazy-initialized on first use
- **Blocking API**: All async functions are exposed as blocking via `RUNTIME.block_on()`
- **Memory management**: Strings returned to the caller are heap-allocated C strings with automatic cleanup
- **Thread-safe**: All singleton instances are protected by mutexes

## Building

```bash
# Debug build
cargo build -p pubky-sdk-ffi

# Release build (recommended)
cargo build -p pubky-sdk-ffi --release
```

The compiled library will be at:
- **Linux**: `target/release/libpubky_sdk_ffi.so`
- **macOS**: `target/release/libpubky_sdk_ffi.dylib`
- **Windows**: `target/release/pubky_sdk_ffi.dll`

## API

All functions return JSON strings that must be freed with `pubky_string_free()`.

### Response Format

```json
// Success
{
  "success": true,
  "data": { ... }
}

// Error
{
  "success": false,
  "error": "Error message"
}
```

### Functions

#### Memory Management

- `pubky_string_free(ptr)` - Free a string returned by any FFI function

#### Keypair Operations

- `pubky_keypair_random()` - Generate a new random keypair
- `pubky_keypair_from_secret_key(secret_key_hex)` - Create keypair from hex secret key

#### Authentication

- `pubky_signup(secret_key_hex, homeserver_pubkey, signup_token, testnet)` - Sign up new user
- `pubky_signin(secret_key_hex, testnet)` - Sign in existing user

#### Storage Operations

- `pubky_put(secret_key_hex, path, content, testnet)` - Write data
- `pubky_get(secret_key_hex, path, testnet)` - Read data
- `pubky_delete(secret_key_hex, path, testnet)` - Delete data

#### Public Storage

- `pubky_public_get(address, testnet)` - Read public data (no auth)
- `pubky_public_list(address, limit, testnet)` - List directory entries

#### HTTP Client

- `pubky_http_request(method, url, body, headers_json, testnet)` - Make HTTP request

#### PKDNS

- `pubky_resolve_homeserver(public_key, testnet)` - Resolve user's homeserver

#### Auth Flow

- `pubky_auth_start(capabilities, testnet)` - Start auth flow for keyless apps

#### Utilities

- `pubky_version()` - Get library version
- `pubky_resolve_address(address)` - Resolve pubky address to transport URL

## Language Bindings

### Ruby on Rails

See [RAILS_INTEGRATION.md](RAILS_INTEGRATION.md) for detailed Ruby/Rails integration guide.

### Other Languages

The FFI uses standard C calling conventions, making it compatible with any language that supports C FFI:

- **Python**: Use `ctypes` or `cffi`
- **Node.js**: Use `ffi-napi`
- **Go**: Use `cgo`
- **C#/.NET**: Use P/Invoke
- **Java**: Use JNI or JNA

## Example (C pseudocode)

```c
#include <stdio.h>
#include <stdlib.h>

// Declare FFI functions
extern char* pubky_keypair_random();
extern void pubky_string_free(char* ptr);

int main() {
    char* result = pubky_keypair_random();
    printf("Result: %s\n", result);
    pubky_string_free(result);
    return 0;
}
```

## License

MIT License
