# leon-link-native

Leon's relay client for React Native. It is the Rust crate
[`crates/leon-mobile`](../../../crates/leon-mobile) (pairing, the end-to-end
encrypted session, agent turns) exposed as a Turbo Module by
[uniffi-bindgen-react-native](https://github.com/jhugman/uniffi-bindgen-react-native)
0.31.0-6, which pairs with `uniffi` 0.31 in the crate. Nothing of the protocol
is written in TypeScript.

```ts
import { LeonMobile, checkCode, Permission } from 'leon-link-native';
```

## What is committed and what is built

| Path | What | Committed |
| --- | --- | --- |
| `src/generated`, `cpp/generated` | the bindings, generated from the crate | yes |
| `src/index.tsx`, `src/NativeLeonLinkNative.ts`, `cpp/*`, `android/*`, `ios/*`, `*.podspec` | the Turbo Module glue, generated | yes |
| `android/src/main/jniLibs`, `*.xcframework` | the Rust library per phone architecture | no, built |

Do not edit generated files: change the crate and generate again. The
generated TypeScript imports its runtime from `@ubjs/core`, which is pinned to
the same version as `uniffi-bindgen-react-native`.

## Commands

```sh
(cd ../.. && bun install)   # the module is a workspace of mobile/: one install, one lockfile
bun run ubrn:generate   # after changing the crate's API: bindings + glue, from a desktop build
bun run ubrn:android    # Rust for arm64-v8a and x86_64, into android/src/main/jniLibs
bun run ubrn:ios        # Rust for device and simulator, into the xcframework (macOS only)
bun run typecheck
```

`ubrn:generate` needs only Rust. `ubrn:android` needs the Android NDK
(`ANDROID_NDK_HOME`), `cargo install cargo-ndk` and
`rustup target add aarch64-linux-android x86_64-linux-android`. `ubrn:ios`
needs Xcode and `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`.
Add `:release` to either for an optimised library.

The module needs React Native's New Architecture and a development build; it
cannot run in Expo Go.

## A relay to test against

```sh
cargo run -p leon-mobile --example dev-relay -- 0.0.0.0:8787
leon host --pair --relay ws://<this computer>:8787
```

Give the phone `ws://<this computer>:8787` as its relay and the code the host
prints. The development relay has no TLS and no limits; it is for a local
network only.
