# Pubky SDK Integration with Ruby on Rails via FFI

This guide explains how to integrate the Pubky SDK into a Ruby on Rails application using FFI (Foreign Function Interface).

## Prerequisites

- Ruby 2.7+ with Rails 6.0+
- Rust toolchain (for building the FFI library)
- `ffi` gem

## Installation

### 1. Build the Pubky FFI Library

From the repository root directory:

```bash
cargo build --release --package pubky-sdk-ffi
```

This creates the shared library:
- **Linux**: `target/release/libpubky_sdk_ffi.so`
- **macOS**: `target/release/libpubky_sdk_ffi.dylib`
- **Windows**: `target/release/pubky_sdk_ffi.dll`

Copy the library to a location accessible by your Rails app (e.g., `lib/` or a system path).

### 2. Add the FFI Gem

Add to your `Gemfile`:

```ruby
gem 'ffi'
```

Then run:

```bash
bundle install
```

### 3. Create the Ruby FFI Wrapper

Create `app/lib/pubky.rb` (or `lib/pubky.rb`):

```ruby
# frozen_string_literal: true

require 'ffi'
require 'json'

module Pubky
  extend FFI::Library

  # Load the shared library
  lib_path = case RUBY_PLATFORM
             when /darwin/
               Rails.root.join('lib', 'libpubky_sdk_ffi.dylib')
             when /linux/
               Rails.root.join('lib', 'libpubky_sdk_ffi.so')
             when /mingw|mswin/
               Rails.root.join('lib', 'pubky_sdk_ffi.dll')
             else
               raise "Unsupported platform: #{RUBY_PLATFORM}"
             end

  ffi_lib lib_path

  # Result structures
  class FfiResult < FFI::Struct
    layout :data, :pointer,
           :error, :pointer,
           :code, :int32
  end

  class FfiBytesResult < FFI::Struct
    layout :data, :pointer,
           :len, :size_t,
           :error, :pointer,
           :code, :int32
  end

  # Initialization
  attach_function :pubky_init, [], :int32

  # Memory management
  attach_function :pubky_string_free, [:pointer], :void
  attach_function :pubky_bytes_free, [:pointer, :size_t], :void

  # Keypair operations
  attach_function :pubky_keypair_random, [], :pointer
  attach_function :pubky_keypair_from_secret_key, [:pointer, :size_t], :pointer
  attach_function :pubky_keypair_secret_key, [:pointer], FfiBytesResult.by_value
  attach_function :pubky_keypair_public_key, [:pointer], :pointer
  attach_function :pubky_keypair_free, [:pointer], :void
  attach_function :pubky_keypair_create_recovery_file, [:pointer, :string], FfiBytesResult.by_value
  attach_function :pubky_keypair_from_recovery_file, [:pointer, :size_t, :string], :pointer

  # PublicKey operations
  attach_function :pubky_public_key_z32, [:pointer], FfiResult.by_value
  attach_function :pubky_public_key_bytes, [:pointer], FfiBytesResult.by_value
  attach_function :pubky_public_key_from_z32, [:string], :pointer
  attach_function :pubky_public_key_free, [:pointer], :void

  # Pubky facade
  attach_function :pubky_new, [], :pointer
  attach_function :pubky_testnet, [], :pointer
  attach_function :pubky_free, [:pointer], :void
  attach_function :pubky_signer, [:pointer, :pointer], :pointer
  attach_function :pubky_public_storage, [:pointer], :pointer
  attach_function :pubky_get_homeserver_of, [:pointer, :pointer], FfiResult.by_value

  # Signer operations
  attach_function :pubky_signer_public_key, [:pointer], :pointer
  attach_function :pubky_signer_signup, [:pointer, :pointer, :string], :pointer
  attach_function :pubky_signer_signin, [:pointer], :pointer
  attach_function :pubky_signer_signin_blocking, [:pointer], :pointer
  attach_function :pubky_signer_free, [:pointer], :void

  # Session operations
  attach_function :pubky_session_public_key, [:pointer], :pointer
  attach_function :pubky_session_capabilities, [:pointer], FfiResult.by_value
  attach_function :pubky_session_storage, [:pointer], :pointer
  attach_function :pubky_session_signout, [:pointer], FfiResult.by_value
  attach_function :pubky_session_revalidate, [:pointer], FfiResult.by_value
  attach_function :pubky_session_free, [:pointer], :void

  # Session storage operations
  attach_function :pubky_session_storage_get_text, [:pointer, :string], FfiResult.by_value
  attach_function :pubky_session_storage_get_bytes, [:pointer, :string], FfiBytesResult.by_value
  attach_function :pubky_session_storage_get_json, [:pointer, :string], FfiResult.by_value
  attach_function :pubky_session_storage_put_text, [:pointer, :string, :string], FfiResult.by_value
  attach_function :pubky_session_storage_put_bytes, [:pointer, :string, :pointer, :size_t], FfiResult.by_value
  attach_function :pubky_session_storage_put_json, [:pointer, :string, :string], FfiResult.by_value
  attach_function :pubky_session_storage_delete, [:pointer, :string], FfiResult.by_value
  attach_function :pubky_session_storage_exists, [:pointer, :string], FfiResult.by_value
  attach_function :pubky_session_storage_list, [:pointer, :string, :uint16], FfiResult.by_value
  attach_function :pubky_session_storage_free, [:pointer], :void

  # Public storage operations
  attach_function :pubky_public_storage_get_text, [:pointer, :string], FfiResult.by_value
  attach_function :pubky_public_storage_get_bytes, [:pointer, :string], FfiBytesResult.by_value
  attach_function :pubky_public_storage_get_json, [:pointer, :string], FfiResult.by_value
  attach_function :pubky_public_storage_exists, [:pointer, :string], FfiResult.by_value
  attach_function :pubky_public_storage_list, [:pointer, :string, :uint16], FfiResult.by_value
  attach_function :pubky_public_storage_free, [:pointer], :void

  # Initialize on load
  pubky_init

  # Error class
  class Error < StandardError
    attr_reader :code

    def initialize(message, code = -1)
      @code = code
      super(message)
    end
  end

  # Helper to handle FfiResult
  def self.handle_result(result, free_data: true)
    if result[:code] != 0
      error_msg = result[:error].null? ? 'Unknown error' : result[:error].read_string
      pubky_string_free(result[:error]) unless result[:error].null?
      raise Error.new(error_msg, result[:code])
    end

    return nil if result[:data].null?

    data = result[:data].read_string
    pubky_string_free(result[:data]) if free_data
    data
  end

  # Helper to handle FfiBytesResult
  def self.handle_bytes_result(result)
    if result[:code] != 0
      error_msg = result[:error].null? ? 'Unknown error' : result[:error].read_string
      pubky_string_free(result[:error]) unless result[:error].null?
      raise Error.new(error_msg, result[:code])
    end

    return nil if result[:data].null? || result[:len] == 0

    data = result[:data].read_bytes(result[:len])
    pubky_bytes_free(result[:data], result[:len])
    data
  end

  # High-level wrapper classes

  class Keypair
    attr_reader :ptr

    def initialize(ptr)
      raise Error.new('Failed to create keypair') if ptr.null?
      @ptr = ptr
      ObjectSpace.define_finalizer(self, self.class.release(@ptr))
    end

    def self.release(ptr)
      proc { Pubky.pubky_keypair_free(ptr) }
    end

    # Generate a random keypair
    def self.random
      new(Pubky.pubky_keypair_random)
    end

    # Create from a 32-byte secret key
    def self.from_secret_key(secret_key)
      raise ArgumentError, 'Secret key must be 32 bytes' unless secret_key.bytesize == 32

      secret_ptr = FFI::MemoryPointer.new(:uint8, 32)
      secret_ptr.put_bytes(0, secret_key)
      new(Pubky.pubky_keypair_from_secret_key(secret_ptr, 32))
    end

    # Create from a recovery file
    def self.from_recovery_file(data, passphrase)
      data_ptr = FFI::MemoryPointer.new(:uint8, data.bytesize)
      data_ptr.put_bytes(0, data)
      new(Pubky.pubky_keypair_from_recovery_file(data_ptr, data.bytesize, passphrase))
    end

    # Get the secret key (32 bytes)
    def secret_key
      Pubky.handle_bytes_result(Pubky.pubky_keypair_secret_key(@ptr))
    end

    # Get the public key
    def public_key
      PublicKey.new(Pubky.pubky_keypair_public_key(@ptr))
    end

    # Create an encrypted recovery file
    def create_recovery_file(passphrase)
      Pubky.handle_bytes_result(Pubky.pubky_keypair_create_recovery_file(@ptr, passphrase))
    end
  end

  class PublicKey
    attr_reader :ptr

    def initialize(ptr)
      raise Error.new('Failed to create public key') if ptr.null?
      @ptr = ptr
      ObjectSpace.define_finalizer(self, self.class.release(@ptr))
    end

    def self.release(ptr)
      proc { Pubky.pubky_public_key_free(ptr) }
    end

    # Create from z-base32 string
    def self.from_z32(z32)
      new(Pubky.pubky_public_key_from_z32(z32))
    end

    # Get z-base32 string representation
    def z32
      Pubky.handle_result(Pubky.pubky_public_key_z32(@ptr))
    end

    # Get raw bytes
    def bytes
      Pubky.handle_bytes_result(Pubky.pubky_public_key_bytes(@ptr))
    end

    def to_s
      z32
    end
  end

  class Client
    attr_reader :ptr

    def initialize(testnet: false)
      @ptr = testnet ? Pubky.pubky_testnet : Pubky.pubky_new
      raise Error.new('Failed to create Pubky client') if @ptr.null?
      ObjectSpace.define_finalizer(self, self.class.release(@ptr))
    end

    def self.release(ptr)
      proc { Pubky.pubky_free(ptr) }
    end

    # Create a signer from a keypair
    def signer(keypair)
      Signer.new(Pubky.pubky_signer(@ptr, keypair.ptr), keypair)
    end

    # Get public storage
    def public_storage
      PublicStorage.new(Pubky.pubky_public_storage(@ptr))
    end

    # Resolve homeserver for a public key
    def get_homeserver_of(public_key)
      result = Pubky.pubky_get_homeserver_of(@ptr, public_key.ptr)
      z32 = Pubky.handle_result(result)
      z32 ? PublicKey.from_z32(z32) : nil
    end
  end

  class Signer
    attr_reader :ptr, :keypair

    def initialize(ptr, keypair)
      raise Error.new('Failed to create signer') if ptr.null?
      @ptr = ptr
      @keypair = keypair
      ObjectSpace.define_finalizer(self, self.class.release(@ptr))
    end

    def self.release(ptr)
      proc { Pubky.pubky_signer_free(ptr) }
    end

    # Get public key
    def public_key
      PublicKey.new(Pubky.pubky_signer_public_key(@ptr))
    end

    # Sign up at a homeserver
    def signup(homeserver, token = nil)
      session_ptr = Pubky.pubky_signer_signup(@ptr, homeserver.ptr, token)
      Session.new(session_ptr)
    end

    # Sign in (for returning users)
    def signin
      session_ptr = Pubky.pubky_signer_signin(@ptr)
      Session.new(session_ptr)
    end

    # Sign in blocking (waits for PKDNS publish)
    def signin_blocking
      session_ptr = Pubky.pubky_signer_signin_blocking(@ptr)
      Session.new(session_ptr)
    end
  end

  class Session
    attr_reader :ptr

    def initialize(ptr)
      raise Error.new('Failed to create session') if ptr.null?
      @ptr = ptr
      @freed = false
      ObjectSpace.define_finalizer(self, self.class.release(@ptr))
    end

    def self.release(ptr)
      proc { Pubky.pubky_session_free(ptr) unless ptr.null? }
    end

    # Get public key
    def public_key
      PublicKey.new(Pubky.pubky_session_public_key(@ptr))
    end

    # Get capabilities
    def capabilities
      Pubky.handle_result(Pubky.pubky_session_capabilities(@ptr))
    end

    # Get session storage
    def storage
      SessionStorage.new(Pubky.pubky_session_storage(@ptr))
    end

    # Sign out (invalidates the session)
    def signout
      result = Pubky.pubky_session_signout(@ptr)
      @freed = true
      Pubky.handle_result(result)
    end

    # Revalidate session
    def revalidate
      result = Pubky.pubky_session_revalidate(@ptr)
      Pubky.handle_result(result)
    end
  end

  class SessionStorage
    attr_reader :ptr

    def initialize(ptr)
      raise Error.new('Failed to create session storage') if ptr.null?
      @ptr = ptr
      ObjectSpace.define_finalizer(self, self.class.release(@ptr))
    end

    def self.release(ptr)
      proc { Pubky.pubky_session_storage_free(ptr) }
    end

    # Get text from path
    def get(path)
      Pubky.handle_result(Pubky.pubky_session_storage_get_text(@ptr, path))
    end

    # Get bytes from path
    def get_bytes(path)
      Pubky.handle_bytes_result(Pubky.pubky_session_storage_get_bytes(@ptr, path))
    end

    # Get JSON from path
    def get_json(path)
      json_str = Pubky.handle_result(Pubky.pubky_session_storage_get_json(@ptr, path))
      JSON.parse(json_str)
    end

    # Put text at path
    def put(path, body)
      Pubky.handle_result(Pubky.pubky_session_storage_put_text(@ptr, path, body))
    end

    # Put bytes at path
    def put_bytes(path, body)
      body_ptr = FFI::MemoryPointer.new(:uint8, body.bytesize)
      body_ptr.put_bytes(0, body)
      Pubky.handle_result(Pubky.pubky_session_storage_put_bytes(@ptr, path, body_ptr, body.bytesize))
    end

    # Put JSON at path
    def put_json(path, value)
      json_str = value.is_a?(String) ? value : JSON.generate(value)
      Pubky.handle_result(Pubky.pubky_session_storage_put_json(@ptr, path, json_str))
    end

    # Delete path
    def delete(path)
      Pubky.handle_result(Pubky.pubky_session_storage_delete(@ptr, path))
    end

    # Check if path exists
    def exists?(path)
      result = Pubky.handle_result(Pubky.pubky_session_storage_exists(@ptr, path))
      result == 'true'
    end

    # List directory contents
    def list(path, limit: 100)
      json_str = Pubky.handle_result(Pubky.pubky_session_storage_list(@ptr, path, limit))
      JSON.parse(json_str)
    end
  end

  class PublicStorage
    attr_reader :ptr

    def initialize(ptr)
      raise Error.new('Failed to create public storage') if ptr.null?
      @ptr = ptr
      ObjectSpace.define_finalizer(self, self.class.release(@ptr))
    end

    def self.release(ptr)
      proc { Pubky.pubky_public_storage_free(ptr) }
    end

    # Get text from address
    def get(address)
      Pubky.handle_result(Pubky.pubky_public_storage_get_text(@ptr, address))
    end

    # Get bytes from address
    def get_bytes(address)
      Pubky.handle_bytes_result(Pubky.pubky_public_storage_get_bytes(@ptr, address))
    end

    # Get JSON from address
    def get_json(address)
      json_str = Pubky.handle_result(Pubky.pubky_public_storage_get_json(@ptr, address))
      JSON.parse(json_str)
    end

    # Check if address exists
    def exists?(address)
      result = Pubky.handle_result(Pubky.pubky_public_storage_exists(@ptr, address))
      result == 'true'
    end

    # List directory contents
    def list(address, limit: 100)
      json_str = Pubky.handle_result(Pubky.pubky_public_storage_list(@ptr, address, limit))
      JSON.parse(json_str)
    end
  end
end
```

## Quick Start Examples

### 1. Generate a Keypair

```ruby
# Generate a random keypair
keypair = Pubky::Keypair.random
public_key = keypair.public_key

puts "Public Key: #{public_key.z32}"

# Save secret key for later
secret_key = keypair.secret_key  # 32 bytes
```

### 2. Create from Secret Key

```ruby
# Restore keypair from secret key
secret_key = "\x00" * 32  # Your 32-byte secret key
keypair = Pubky::Keypair.from_secret_key(secret_key)
```

### 3. Sign Up on a Homeserver

```ruby
# Create a Pubky client
pubky = Pubky::Client.new

# Generate keypair and create signer
keypair = Pubky::Keypair.random
signer = pubky.signer(keypair)

# Sign up on a homeserver (identified by its public key)
homeserver = Pubky::PublicKey.from_z32("o4dksf...uyy")
session = signer.signup(homeserver)

puts "Signed up as: #{session.public_key.z32}"
```

### 4. Read/Write Data (Session Storage)

```ruby
# Get storage handle from session
storage = session.storage

# Write text data
storage.put("/pub/my-cool-app/hello.txt", "hello world")

# Read text data
body = storage.get("/pub/my-cool-app/hello.txt")
puts body  # => "hello world"

# Write JSON data
storage.put_json("/pub/my-cool-app/data.json", { name: "Alice", score: 100 })

# Read JSON data
data = storage.get_json("/pub/my-cool-app/data.json")
puts data["name"]  # => "Alice"

# Check if file exists
if storage.exists?("/pub/my-cool-app/hello.txt")
  puts "File exists!"
end

# List directory contents
entries = storage.list("/pub/my-cool-app/", limit: 10)
entries.each { |url| puts url }

# Delete a file
storage.delete("/pub/my-cool-app/hello.txt")
```

### 5. Public Read (Unauthenticated)

```ruby
pubky = Pubky::Client.new
public_storage = pubky.public_storage

# Read another user's public file
user_id = "operrr8wsbpr3ue9d4qj41ge1kcc6r7fdiy6o3ugjrrhi4y77rdo"
address = "pubky#{user_id}/pub/my-cool-app/hello.txt"

text = public_storage.get(address)
puts text

# List a user's public directory
entries = public_storage.list("pubky#{user_id}/pub/my-cool-app/", limit: 10)
entries.each { |url| puts url }
```

### 6. Sign In (Returning User)

```ruby
pubky = Pubky::Client.new

# Load keypair from saved secret key
keypair = Pubky::Keypair.from_secret_key(saved_secret_key)
signer = pubky.signer(keypair)

# Sign in
session = signer.signin

# Now you can use the storage
storage = session.storage
# ...
```

### 7. Recovery File

```ruby
# Create an encrypted recovery file
keypair = Pubky::Keypair.random
recovery_data = keypair.create_recovery_file("my-secure-passphrase")

# Save to file
File.binwrite("alice.pkarr", recovery_data)

# Later: restore keypair from recovery file
recovery_data = File.binread("alice.pkarr")
keypair = Pubky::Keypair.from_recovery_file(recovery_data, "my-secure-passphrase")
```

### 8. Resolve Homeserver

```ruby
pubky = Pubky::Client.new

# Get another user's homeserver
user = Pubky::PublicKey.from_z32("operrr8wsbpr3ue9d4qj41ge1kcc6r7fdiy6o3ugjrrhi4y77rdo")
homeserver = pubky.get_homeserver_of(user)

if homeserver
  puts "User's homeserver: #{homeserver.z32}"
else
  puts "Homeserver not found"
end
```

## Rails Integration Examples

### Service Object Pattern

```ruby
# app/services/pubky_service.rb
class PubkyService
  def initialize(testnet: Rails.env.development?)
    @client = Pubky::Client.new(testnet: testnet)
  end

  def create_user
    keypair = Pubky::Keypair.random
    {
      public_key: keypair.public_key.z32,
      secret_key: keypair.secret_key
    }
  end

  def sign_in(secret_key)
    keypair = Pubky::Keypair.from_secret_key(secret_key)
    signer = @client.signer(keypair)
    signer.signin
  end

  def get_public_file(user_id, path)
    address = "pubky#{user_id}#{path}"
    @client.public_storage.get(address)
  rescue Pubky::Error => e
    Rails.logger.error("Pubky error: #{e.message}")
    nil
  end
end
```

### Controller Example

```ruby
# app/controllers/pubky_controller.rb
class PubkyController < ApplicationController
  before_action :set_pubky_service

  def read_public_file
    user_id = params[:user_id]
    path = params[:path]

    content = @pubky.get_public_file(user_id, "/pub/#{path}")

    if content
      render plain: content
    else
      head :not_found
    end
  end

  private

  def set_pubky_service
    @pubky = PubkyService.new
  end
end
```

### Background Job Example

```ruby
# app/jobs/pubky_sync_job.rb
class PubkySyncJob < ApplicationJob
  queue_as :default

  def perform(user_id, path, content)
    service = PubkyService.new
    # Load user's secret key from secure storage
    secret_key = CredentialStore.get_secret_key(user_id)

    session = service.sign_in(secret_key)
    session.storage.put(path, content)
    session.signout
  end
end
```

## Thread Safety

The Pubky FFI bindings use a global Tokio runtime for async operations. The runtime is initialized automatically on first use. For thread-safety in Rails:

- Each `Pubky::Client` instance can be safely shared across threads
- `Session` and `Storage` objects should be created per-request
- Use connection pools or create new clients as needed

## Error Handling

```ruby
begin
  storage.put("/pub/app/file.txt", "content")
rescue Pubky::Error => e
  case e.code
  when 1
    # Request error (network/server)
    Rails.logger.error("Network error: #{e.message}")
  when 4
    # Authentication error
    Rails.logger.error("Auth error: #{e.message}")
  else
    Rails.logger.error("Pubky error (#{e.code}): #{e.message}")
  end
end
```

## Testing

For testing, use the testnet mode:

```ruby
# spec/support/pubky_helper.rb
RSpec.configure do |config|
  config.before(:each, :pubky) do
    @pubky = Pubky::Client.new(testnet: true)
  end
end
```

```ruby
# spec/services/pubky_service_spec.rb
RSpec.describe PubkyService, :pubky do
  it "creates and reads files" do
    keypair = Pubky::Keypair.random
    signer = @pubky.signer(keypair)

    # Note: This requires a running testnet
    # Run: cargo run --package pubky-testnet
    session = signer.signin
    storage = session.storage

    storage.put("/pub/test/hello.txt", "test content")
    content = storage.get("/pub/test/hello.txt")

    expect(content).to eq("test content")
  end
end
```

## See Also

- [Pubky SDK README](../../README.md) - Main SDK documentation
- [FFI README](./README.md) - General FFI documentation
- [Pubky Core Examples](https://github.com/pubky/pubky-core/tree/main/examples) - More examples

---

**License:** MIT
