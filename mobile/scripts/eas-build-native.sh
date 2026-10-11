#!/usr/bin/env bash
# Runs on an EAS builder: builds the Rust library of modules/leon-link-native
# for the platform being built, because the libraries are not committed.
# Locally, use `bun run native:android` or `bun run native:ios` instead.
#
# Called twice, as `pre` (before the dependencies are installed) and `post`
# (after them). Android is built in `post`. iOS has to be built in `pre`:
# there the `post` hook runs after `pod install`, which already needs the
# framework and the generated sources, so the dependencies are installed
# here first.
set -euo pipefail

when="${1:-post}"
platform="${EAS_BUILD_PLATFORM:-}"
root="$(cd "$(dirname "$0")/.." && pwd)"

case "$platform:$when" in
  android:post | ios:pre) ;;
  android:pre | ios:post) exit 0 ;;
  *)
    echo "EAS_BUILD_PLATFORM is not set: nothing to build." >&2
    exit 0
    ;;
esac

if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi

suffix=""
if [ "${LEON_NATIVE_PROFILE:-release}" = "release" ]; then
  suffix=":release"
fi

cd "$root"
if [ "$when" = "pre" ]; then
  bun install --frozen-lockfile
fi
cd modules/leon-link-native

case "$platform" in
  android)
    rustup target add aarch64-linux-android x86_64-linux-android
    command -v cargo-ndk >/dev/null 2>&1 || cargo install cargo-ndk --locked
    npm run "ubrn:android${suffix}"
    ;;
  ios)
    rustup target add aarch64-apple-ios aarch64-apple-ios-sim
    npm run "ubrn:ios${suffix}"
    ;;
esac
