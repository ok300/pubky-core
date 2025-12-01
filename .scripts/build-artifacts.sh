#!/bin/bash

# -------------------------------------------------------------------------------------------------
# This script prepares the artifacts for the current project.
# It builds all the binaries and prepares them for upload as a Github Release.
# The end result will be a target/github-release directory with the following structure:
#
# target/github-release/
# ├── pubky-core-v0.5.0-rc.0-linux-arm64.tar.gz
# ├── pubky-core-v0.5.0-rc.0-linux-amd64.tar.gz
# ├── pubky-core-v0.5.0-rc.0-windows-amd64.tar.gz
# ├── pubky-core-v0.5.0-rc.0-osx-arm64.tar.gz
# ├── pubky-core-v0.5.0-rc.0-osx-amd64.tar.gz
# └── ...
#
# Usage:
#   ./build-artifacts.sh                    # Build all targets (requires cross and macOS docker images)
#   ./build-artifacts.sh linux-windows      # Build only Linux and Windows targets using cross
#   ./build-artifacts.sh macos <target> <nickname>  # Build a single macOS target natively
#
# Make sure you installed https://github.com/cross-rs/cross for cross-compilation.
# -------------------------------------------------------------------------------------------------


set -e # fail the script if any command fails
set -u # fail the script if any variable is not set
set -o pipefail # fail the script if any pipe command fails


# Read the version from the homeserver
VERSION=$(cargo pkgid -p pubky-homeserver | awk -F# '{print $NF}')
echo "Preparing release executables for version $VERSION..."

# List of binaries to build.
ARTIFACTS=("pubky-homeserver")

echo "Create the github-release directory..."
mkdir -p target/github-release

# Helper function to build an artifact for one specific target using cross.
build_target_cross() {
    local TARGET=$1
    local NICKNAME=$2
    echo "Build $NICKNAME with $TARGET (using cross)"
    FOLDER="pubky-core-v$VERSION-$NICKNAME"
    DICT="target/github-release/$FOLDER"
    mkdir -p $DICT
    for ARTIFACT in "${ARTIFACTS[@]}"; do
        echo "- Build $ARTIFACT with $TARGET"
        cross build -p $ARTIFACT --release --target $TARGET
        if [[ $TARGET == *"windows"* ]]; then
            cp target/$TARGET/release/$ARTIFACT.exe $DICT/
        else
            cp target/$TARGET/release/$ARTIFACT $DICT/
        fi
        echo "[Done] Artifact $ARTIFACT built for $TARGET"
    done;
    (cd target/github-release && tar -czf $FOLDER.tar.gz $FOLDER && rm -rf $FOLDER)
}

# Helper function to build an artifact for one specific target using cargo (native).
build_target_native() {
    local TARGET=$1
    local NICKNAME=$2
    echo "Build $NICKNAME with $TARGET (native)"
    FOLDER="pubky-core-v$VERSION-$NICKNAME"
    DICT="target/github-release/$FOLDER"
    mkdir -p $DICT
    for ARTIFACT in "${ARTIFACTS[@]}"; do
        echo "- Build $ARTIFACT with $TARGET"
        cargo build -p $ARTIFACT --release --target $TARGET
        cp target/$TARGET/release/$ARTIFACT $DICT/
        echo "[Done] Artifact $ARTIFACT built for $TARGET"
    done;
    (cd target/github-release && tar -czf $FOLDER.tar.gz $FOLDER && rm -rf $FOLDER)
}

# Parse command line arguments
BUILD_MODE="${1:-all}"

case "$BUILD_MODE" in
    "linux-windows")
        # Check if cross is installed
        if ! command -v cross &> /dev/null; then
            echo "cross executable could not be found. It is required to cross-compile the binaries. Please install it from https://github.com/cross-rs/cross"
            exit 1
        fi

        TARGETS=(
            "aarch64-unknown-linux-musl,linux-arm64"
            "x86_64-unknown-linux-musl,linux-amd64"
            "x86_64-pc-windows-gnu,windows-amd64"
        )

        echo "Build Linux and Windows binaries for version $VERSION..."
        for ELEMENT in "${TARGETS[@]}"; do
            IFS=',' read -r TARGET NICKNAME <<< "$ELEMENT"
            build_target_cross $TARGET $NICKNAME
        done
        ;;

    "macos")
        # Build a single macOS target natively
        TARGET="${2:-}"
        NICKNAME="${3:-}"
        if [[ -z "$TARGET" || -z "$NICKNAME" ]]; then
            echo "Usage: $0 macos <target> <nickname>"
            echo "Example: $0 macos aarch64-apple-darwin osx-arm64"
            exit 1
        fi
        echo "Build macOS binary for version $VERSION..."
        build_target_native $TARGET $NICKNAME
        ;;

    "all")
        # Check if cross is installed
        if ! command -v cross &> /dev/null; then
            echo "cross executable could not be found. It is required to cross-compile the binaries. Please install it from https://github.com/cross-rs/cross"
            exit 1
        fi

        TARGETS=(
            "aarch64-unknown-linux-musl,linux-arm64"
            "x86_64-unknown-linux-musl,linux-amd64"
            "x86_64-pc-windows-gnu,windows-amd64"
            "aarch64-apple-darwin,osx-arm64"
            "x86_64-apple-darwin,osx-amd64"
        )

        echo "Build all the binaries for version $VERSION..."
        for ELEMENT in "${TARGETS[@]}"; do
            IFS=',' read -r TARGET NICKNAME <<< "$ELEMENT"
            build_target_cross $TARGET $NICKNAME
        done
        ;;

    *)
        echo "Unknown build mode: $BUILD_MODE"
        echo "Usage: $0 [linux-windows|macos <target> <nickname>|all]"
        exit 1
        ;;
esac

tree target/github-release || ls -la target/github-release
(cd target/github-release && pwd)
