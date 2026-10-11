#!/usr/bin/env bash
# Runs on an EAS builder after dependencies are installed: builds the Rust
# library of modules/leon-link-native for the platform being built, because
# the libraries are not committed. Locally, use `bun run native:android` or
# `bun run native:ios` instead.
set -euo pipefail

cd "$(dirname "$0")/../modules/leon-link-native"

if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi

suffix=""
if [ "${LEON_NATIVE_PROFILE:-release}" = "release" ]; then
  suffix=":release"
fi

case "${EAS_BUILD_PLATFORM:-}" in
  android)
    rustup target add aarch64-linux-android x86_64-linux-android
    command -v cargo-ndk >/dev/null 2>&1 || cargo install cargo-ndk --locked
    npm run "ubrn:android${suffix}"
    ;;
  ios)
    rustup target add aarch64-apple-ios aarch64-apple-ios-sim
    npm run "ubrn:ios${suffix}"
    ;;
  *)
    echo "EAS_BUILD_PLATFORM is not set: nothing to build." >&2
    ;;
esac
