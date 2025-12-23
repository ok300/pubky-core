# Pubky SDK FFI for Ruby on Rails

This document describes how to integrate the Pubky SDK in a Ruby on Rails application via FFI (Foreign Function Interface).

## Prerequisites

1. **Rust toolchain**: Install from [rustup.rs](https://rustup.rs/)
2. **Ruby FFI gem**: Add to your `Gemfile`
3. **Build the FFI library**: Compile the `pubky-sdk-ffi` crate

## Installation

### Step 1: Build the FFI Library

```bash
# Clone the repository
git clone https://github.com/pubky/pubky-core.git
cd pubky-core

# Build the FFI library in release mode
cargo build -p pubky-sdk-ffi --release

# The compiled library will be at:
# - Linux: target/release/libpubky_sdk_ffi.so
# - macOS: target/release/libpubky_sdk_ffi.dylib
# - Windows: target/release/pubky_sdk_ffi.dll
```

### Step 2: Add the FFI Gem to Your Rails App

```ruby
# Gemfile
gem 'ffi'
```

Then run:

```bash
bundle install
```

### Step 3: Create the Ruby FFI Module

Create a file at `lib/pubky_sdk_ffi.rb`:

```ruby
# lib/pubky_sdk_ffi.rb
require 'ffi'
require 'json'

module PubkySdkFFI
  extend FFI::Library

  # Load the shared library based on platform
  LIB_NAME = case RbConfig::CONFIG['host_os']
             when /darwin/  then 'libpubky_sdk_ffi.dylib'
             when /linux/   then 'libpubky_sdk_ffi.so'
             when /mswin|mingw/ then 'libpubky_sdk_ffi.dll'
             end

  ffi_lib Rails.root.join('lib', 'native', LIB_NAME).to_s

  # Memory management - strings are automatically freed by the GC callback
  attach_function :pubky_string_free, [:pointer], :void

  # Keypair operations
  attach_function :pubky_keypair_random, [], :pointer
  attach_function :pubky_keypair_from_secret_key, [:string], :pointer

  # Authentication
  attach_function :pubky_signup, [:string, :string, :string, :int], :pointer
  attach_function :pubky_signin, [:string, :int], :pointer

  # Storage operations
  attach_function :pubky_put, [:string, :string, :string, :int], :pointer
  attach_function :pubky_get, [:string, :string, :int], :pointer
  attach_function :pubky_delete, [:string, :string, :int], :pointer

  # Public storage (read-only)
  attach_function :pubky_public_get, [:string, :int], :pointer
  attach_function :pubky_public_list, [:string, :uint16, :int], :pointer

  # HTTP client requests
  attach_function :pubky_http_request, [:string, :string, :string, :string, :int], :pointer

  # PKDNS operations
  attach_function :pubky_resolve_homeserver, [:string, :int], :pointer

  # Auth flow
  attach_function :pubky_auth_start, [:string, :int], :pointer

  # Utility functions
  attach_function :pubky_version, [], :pointer
  attach_function :pubky_resolve_address, [:string], :pointer

  # Helper method to call FFI and parse JSON response
  def self.call_ffi(pointer)
    return nil if pointer.null?

    begin
      json_string = pointer.read_string
      result = JSON.parse(json_string)

      if result['success']
        result['data']
      else
        raise PubkyError, result['error']
      end
    ensure
      pubky_string_free(pointer)
    end
  end

  # Custom error class
  class PubkyError < StandardError; end

  # High-level Ruby wrapper class
  class Client
    def initialize(testnet: false)
      @testnet = testnet ? 1 : 0
    end

    # Generate a new random keypair
    # Returns: { "secret_key" => "hex...", "public_key" => "z32..." }
    def keypair_random
      ptr = PubkySdkFFI.pubky_keypair_random
      PubkySdkFFI.call_ffi(ptr)
    end

    # Create a keypair from a hex-encoded secret key
    # Returns: { "public_key" => "z32..." }
    def keypair_from_secret_key(secret_key_hex)
      ptr = PubkySdkFFI.pubky_keypair_from_secret_key(secret_key_hex)
      PubkySdkFFI.call_ffi(ptr)
    end

    # Sign up a new user on a homeserver
    # Returns: { "public_key" => "z32...", "capabilities" => "..." }
    def signup(secret_key_hex, homeserver_pubkey, signup_token: nil)
      ptr = PubkySdkFFI.pubky_signup(
        secret_key_hex,
        homeserver_pubkey,
        signup_token,
        @testnet
      )
      PubkySdkFFI.call_ffi(ptr)
    end

    # Sign in an existing user
    # Returns: { "public_key" => "z32...", "capabilities" => "..." }
    def signin(secret_key_hex)
      ptr = PubkySdkFFI.pubky_signin(secret_key_hex, @testnet)
      PubkySdkFFI.call_ffi(ptr)
    end

    # Put data to user's storage
    # Returns: nil on success
    def put(secret_key_hex, path, content)
      ptr = PubkySdkFFI.pubky_put(secret_key_hex, path, content, @testnet)
      PubkySdkFFI.call_ffi(ptr)
    end

    # Get data from user's storage
    # Returns: { "content" => "..." }
    def get(secret_key_hex, path)
      ptr = PubkySdkFFI.pubky_get(secret_key_hex, path, @testnet)
      PubkySdkFFI.call_ffi(ptr)
    end

    # Delete data from user's storage
    # Returns: nil on success
    def delete(secret_key_hex, path)
      ptr = PubkySdkFFI.pubky_delete(secret_key_hex, path, @testnet)
      PubkySdkFFI.call_ffi(ptr)
    end

    # Get public data (no authentication needed)
    # Address format: "pubky<public_key>/pub/app/file.txt"
    # Returns: { "content" => "..." }
    def public_get(address)
      ptr = PubkySdkFFI.pubky_public_get(address, @testnet)
      PubkySdkFFI.call_ffi(ptr)
    end

    # List public directory entries
    # Address format: "pubky<public_key>/pub/app/" (must end with /)
    # Returns: { "entries" => [{ "url" => "...", "path" => "...", "owner" => "...", "is_dir" => bool }] }
    def public_list(address, limit: 0)
      ptr = PubkySdkFFI.pubky_public_list(address, limit, @testnet)
      PubkySdkFFI.call_ffi(ptr)
    end

    # Make an HTTP request using the Pubky HTTP client
    # Supports pubky:// URLs and standard https:// URLs
    # Returns: { "status" => 200, "headers" => {...}, "body" => "..." }
    def http_request(method, url, body: nil, headers: nil)
      headers_json = headers ? headers.to_json : nil
      ptr = PubkySdkFFI.pubky_http_request(
        method.to_s.upcase,
        url,
        body,
        headers_json,
        @testnet
      )
      PubkySdkFFI.call_ffi(ptr)
    end

    # Resolve a user's homeserver via PKDNS
    # Returns: { "homeserver" => "z32..." } or { "homeserver" => nil }
    def resolve_homeserver(public_key)
      ptr = PubkySdkFFI.pubky_resolve_homeserver(public_key, @testnet)
      PubkySdkFFI.call_ffi(ptr)
    end

    # Start an auth flow for keyless/QR apps
    # Capabilities format: "/pub/app/:rw,/pub/foo/:r"
    # Returns: { "authorization_url" => "..." }
    def auth_start(capabilities)
      ptr = PubkySdkFFI.pubky_auth_start(capabilities, @testnet)
      PubkySdkFFI.call_ffi(ptr)
    end

    # Get the FFI library version
    def self.version
      ptr = PubkySdkFFI.pubky_version
      return nil if ptr.null?

      begin
        ptr.read_string
      ensure
        PubkySdkFFI.pubky_string_free(ptr)
      end
    end

    # Resolve a pubky address to its transport URL
    # Input: "pubky<pk>/pub/app/file" or "pubky://<pk>/..."
    # Returns: { "url" => "https://..." }
    def self.resolve_address(address)
      ptr = PubkySdkFFI.pubky_resolve_address(address)
      PubkySdkFFI.call_ffi(ptr)
    end
  end
end
```

### Step 4: Configure Rails Autoloading

Add to `config/application.rb`:

```ruby
# config/application.rb
config.autoload_paths << Rails.root.join('lib')
```

## Quick Start Examples

### 1. Create a New Random Keypair

```ruby
client = PubkySdkFFI::Client.new

# Generate a new keypair
keypair = client.keypair_random
puts "Secret Key: #{keypair['secret_key']}"
puts "Public Key: #{keypair['public_key']}"
```

### 2. Sign Up on a Homeserver

```ruby
client = PubkySdkFFI::Client.new

# Generate a keypair
keypair = client.keypair_random
secret_key = keypair['secret_key']

# Sign up on a homeserver (use actual homeserver public key)
homeserver = "o4dksfbqk85ogzdb5osziw6befigbuxmuxkuxq8434q89uj56uyy"
session = client.signup(secret_key, homeserver)
puts "Signed up as: #{session['public_key']}"
```

### 3. Sign In an Existing User

```ruby
client = PubkySdkFFI::Client.new

# Sign in with an existing secret key
secret_key = "your_hex_encoded_secret_key"
session = client.signin(secret_key)
puts "Signed in as: #{session['public_key']}"
```

### 4. Read and Write Data

```ruby
client = PubkySdkFFI::Client.new
secret_key = "your_hex_encoded_secret_key"

# Write data
client.put(secret_key, "/pub/my-cool-app/hello.txt", "Hello, World!")

# Read data back
result = client.get(secret_key, "/pub/my-cool-app/hello.txt")
puts "Content: #{result['content']}"  # => "Hello, World!"

# Delete data
client.delete(secret_key, "/pub/my-cool-app/hello.txt")
```

### 5. Read Public Data (No Authentication)

```ruby
client = PubkySdkFFI::Client.new

# Read another user's public file
user_pubkey = "ihaqcthsdbk751sxctk849bdr7yz7a934qen5gmpcbwcur49i97y"
result = client.public_get("pubky#{user_pubkey}/pub/pubky.app/profile.json")
puts "Profile: #{result['content']}"

# List directory contents
entries = client.public_list("pubky#{user_pubkey}/pub/pubky.app/", limit: 10)
entries['entries'].each do |entry|
  puts "#{entry['path']} (dir: #{entry['is_dir']})"
end
```

### 6. HTTP Client Requests

The Pubky HTTP client can make requests to both pubky:// URLs and standard HTTPS URLs:

```ruby
client = PubkySdkFFI::Client.new

# GET request to a pubky URL
user_pk = "ihaqcthsdbk751sxctk849bdr7yz7a934qen5gmpcbwcur49i97y"
response = client.http_request(
  'GET',
  "https://_pubky.#{user_pk}/pub/pubky.app/profile.json"
)
puts "Status: #{response['status']}"
puts "Body: #{response['body']}"

# POST request with body and headers
response = client.http_request(
  'POST',
  'https://example.com/api/data',
  body: '{"key": "value"}',
  headers: { 'Content-Type' => 'application/json' }
)

# PUT request
response = client.http_request(
  'PUT',
  "https://_pubky.#{user_pk}/pub/my-app/data.json",
  body: '{"updated": true}'
)
```

### 7. Pubky Auth Flow (QR/Deeplink)

For keyless apps that need user authorization:

```ruby
client = PubkySdkFFI::Client.new

# Start an auth flow with read/write capabilities
capabilities = "/pub/example.com/:rw"
flow = client.auth_start(capabilities)

# Display the authorization URL as a QR code or deeplink
puts "Scan to authorize: #{flow['authorization_url']}"

# The user scans this with a Pubky wallet app (e.g., Pubky Ring)
# After approval, you would typically poll or use webhooks to get the session
```

### 8. Resolve PKDNS Records

```ruby
client = PubkySdkFFI::Client.new

# Find a user's homeserver
user_pubkey = "ihaqcthsdbk751sxctk849bdr7yz7a934qen5gmpcbwcur49i97y"
result = client.resolve_homeserver(user_pubkey)

if result['homeserver']
  puts "Homeserver: #{result['homeserver']}"
else
  puts "No homeserver found for this user"
end
```

### 9. Using Testnet Configuration

For development and testing, use the testnet:

```ruby
# Use testnet (requires pubky-testnet running locally)
client = PubkySdkFFI::Client.new(testnet: true)

# All operations will use testnet configuration
keypair = client.keypair_random
# ...
```

## Rails Integration Patterns

### Service Object Pattern

```ruby
# app/services/pubky_service.rb
class PubkyService
  def initialize(testnet: Rails.env.development?)
    @client = PubkySdkFFI::Client.new(testnet: testnet)
  end

  def create_user
    keypair = @client.keypair_random
    # Store secret_key securely (e.g., encrypted in database)
    User.create!(
      pubky_public_key: keypair['public_key'],
      encrypted_secret_key: encrypt(keypair['secret_key'])
    )
  end

  def publish_profile(user, profile_data)
    secret_key = decrypt(user.encrypted_secret_key)
    @client.put(
      secret_key,
      "/pub/#{Rails.application.config.pubky_app_domain}/profile.json",
      profile_data.to_json
    )
  end

  private

  def encrypt(data)
    # Use Rails encryption or your preferred method
    ActiveSupport::MessageEncryptor.new(
      Rails.application.credentials.secret_key_base[0..31]
    ).encrypt_and_sign(data)
  end

  def decrypt(data)
    ActiveSupport::MessageEncryptor.new(
      Rails.application.credentials.secret_key_base[0..31]
    ).decrypt_and_verify(data)
  end
end
```

### Controller Example

```ruby
# app/controllers/profiles_controller.rb
class ProfilesController < ApplicationController
  def show
    client = PubkySdkFFI::Client.new
    result = client.public_get("pubky#{params[:pubkey]}/pub/pubky.app/profile.json")
    @profile = JSON.parse(result['content'])
  rescue PubkySdkFFI::PubkyError => e
    render json: { error: e.message }, status: :not_found
  end

  def update
    service = PubkyService.new
    service.publish_profile(current_user, profile_params)
    redirect_to profile_path, notice: 'Profile updated'
  rescue PubkySdkFFI::PubkyError => e
    flash.now[:alert] = "Failed to update: #{e.message}"
    render :edit
  end
end
```

## Memory Management

The FFI bindings handle memory management automatically:

1. All strings returned by FFI functions are allocated on the Rust heap
2. The `call_ffi` helper method parses the JSON and then calls `pubky_string_free` to deallocate
3. Ruby's garbage collector handles the Ruby-side objects

**Important**: Always use the `call_ffi` helper method or ensure you call `pubky_string_free` on any pointer returned by the FFI functions.

## Error Handling

All FFI functions return JSON responses with a `success` field:

```json
// Success response
{
  "success": true,
  "data": { ... }
}

// Error response
{
  "success": false,
  "error": "Description of what went wrong"
}
```

The `PubkySdkFFI::PubkyError` exception is raised when `success` is `false`.

## Thread Safety

The FFI bindings use singleton pattern with lazy initialization:
- The Pubky facade and HTTP client are initialized on first use
- All instances are protected by mutexes
- Multiple threads can safely call FFI functions concurrently

## Performance Considerations

1. **Connection reuse**: The singleton HTTP client reuses connections
2. **Async to blocking**: All async Rust functions are called via `block_on()`, which may block the calling thread
3. **For high throughput**: Consider using Rails background jobs for Pubky operations

## Troubleshooting

### Library not found

```ruby
# Ensure the library path is correct
ffi_lib '/absolute/path/to/libpubky_sdk_ffi.so'
```

### Symbol not found

Ensure you're using the release build:
```bash
cargo build -p pubky-sdk-ffi --release
```

### Permission denied

```bash
chmod +x target/release/libpubky_sdk_ffi.so
```

## License

MIT License - see the [LICENSE](LICENSE) file for details.
