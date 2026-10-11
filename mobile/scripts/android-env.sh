# Source this to get the Android toolchain on PATH: `. scripts/android-env.sh`.
# Expects JDK 17 from mise (`mise install java@temurin-17`) and the SDK in ~/Android/Sdk.
export JAVA_HOME="${JAVA_HOME:-$(mise where java@temurin-17)}"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.1.12297006}"
export PATH="$JAVA_HOME/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
