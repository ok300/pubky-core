# Pubky SDK FFI for Ruby on Rails

This document describes how to integrate the `pubky-sdk-ffi` into a Ruby on Rails application.

## Prerequisites

- Ruby 2.7+
- Bundler
- A Rust toolchain

## Setup

1.  **Add the `ffi` gem to your `Gemfile`:**

    ```ruby
    gem 'ffi'
    ```

2.  **Install the gem:**

    ```bash
    bundle install
    ```

3.  **Compile the Rust FFI library:**

    Navigate to the `pubky-sdk-ffi` directory and run:

    ```bash
    cargo build
    ```

    This will create a shared library file at `target/debug/libpubky_sdk_ffi.so`.

4.  **Load the `PubkySdkFFI` module:**

    Create an initializer file in your Rails application (e.g., `config/initializers/pubky_sdk.rb`) and add the following code:

    ```ruby
    require_relative 'path/to/pubky-sdk-ffi/lib/pubky_sdk_ffi'
    ```

    Replace `path/to/` with the actual path to the `pubky-sdk-ffi` directory.

## Quickstart

The following examples demonstrate how to use the `PubkySdkFFI` module, mirroring the [Pubky SDK Quickstart guide](https://raw.githubusercontent.com/ok300/pubky-core/refs/heads/main/pubky-sdk/README.md).

```ruby
# 1) Create a new random key user
keypair = PubkySdkFFI::Keypair.random

# 2) Create a signer from the keypair
signer = PubkySdkFFI::Signer.from_keypair(keypair)

# 3) Sign up on a homeserver
homeserver_pk = "dtnb4ush5b1e3iq69j7y4855q667g3s9mt3a554a8gf3xftf1wzrq" # Example public key
session = signer.signup(homeserver_pk)

# 4) Read/Write as the signed-in user
session.storage_put("/pub/my-cool-app/hello.txt", "hello")
body = session.storage_get("/pub/my-cool-app/hello.txt")
puts body #=> "hello"

# Memory management is handled automatically by the garbage collector.
```

### HttpClient Requests

You can also make raw HTTP requests using the `PubkyHttpClient`.

```ruby
# Make a GET request
response = PubkySdkFFI.http_request("GET", "https://example.com")
puts response
```
