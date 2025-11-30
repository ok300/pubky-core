require 'ffi'

module PubkySdkFFI
  extend FFI::Library

  lib_name = case RbConfig::CONFIG['host_os']
             when /darwin|mac os/
               'libpubky_sdk_ffi.dylib'
             when /linux/
               'libpubky_sdk_ffi.so'
             when /mswin|msys|mingw|cygwin|bccwin|wince|emc/
               'pubky_sdk_ffi.dll'
             else
               raise "Unsupported OS: #{RbConfig::CONFIG['host_os']}"
             end

  release_path = File.expand_path("../../../target/release/#{lib_name}", __FILE__)
  debug_path = File.expand_path("../../../target/debug/#{lib_name}", __FILE__)

  ffi_lib [release_path, debug_path]

  # Memory management
  attach_function :free_string, [:pointer], :void

  # Keypair
  attach_function :keypair_random, [], :pointer
  attach_function :keypair_free, [:pointer], :void

  # Signer
  attach_function :signer_from_keypair, [:pointer], :pointer
  attach_function :signer_free, [:pointer], :void
  attach_function :signer_signup, [:pointer, :string], :pointer

  # Session
  attach_function :session_free, [:pointer], :void
  attach_function :session_storage_put, [:pointer, :string, :string], :int
  attach_function :session_storage_get, [:pointer, :string], :pointer

  # HTTP
  attach_function :http_request, [:string, :string], :pointer

  # Wrapper classes
  class Keypair < FFI::ManagedStruct
    layout :_dummy, :pointer

    def self.random
      ptr = PubkySdkFFI.keypair_random
      new(ptr)
    end

    def self.release(ptr)
      PubkySdkFFI.keypair_free(ptr)
    end
  end

  class Signer < FFI::ManagedStruct
    layout :_dummy, :pointer

    def self.from_keypair(keypair)
      ptr = PubkySdkFFI.signer_from_keypair(keypair)
      new(ptr)
    end

    def signup(homeserver_pk)
      session_ptr = PubkySdkFFI.signer_signup(self, homeserver_pk)
      Session.new(session_ptr)
    end

    def self.release(ptr)
      PubkySdkFFI.signer_free(ptr)
    end
  end

  class Session < FFI::ManagedStruct
    layout :_dummy, :pointer

    def storage_put(path, value)
      result = PubkySdkFFI.session_storage_put(self, path, value)
      raise "Failed to put value to storage" if result != 0
    end

    def storage_get(path)
      str_ptr = PubkySdkFFI.session_storage_get(self, path)
      return nil if str_ptr.null?
      str = str_ptr.read_string
      PubkySdkFFI.free_string(str_ptr)
      str
    end

    def self.release(ptr)
      PubkySdkFFI.session_free(ptr)
    end
  end

  def self.http_request(method, url)
    str_ptr = PubkySdkFFI.http_request(method, url)
    str = str_ptr.read_string
    PubkySdkFFI.free_string(str_ptr)
    str
  end
end
