# Tauri Plugin whisper-rs

> README created using AI  
  
> Note: Building Whisper on the Rust side can be a bit of a pain in the ass due to NDK/C++ cross-compilation requirements. 


Installation
```bash
# Using npm
npm run tauri add whisper-rs

# Using pnpm
pnpm tauri add whisper-rs

# Using yarn
yarn tauri add whisper-rs

# Using bun
bun tauri add whisper-rs
```

## Android Requirement

`src-tauri/.cargo/config.toml`
```toml
[env]
ANDROID_NDK_HOME = "~/Android/Sdk/ndk/30.0.14904198"
ANDROID_NDK_ROOT = "~/Android/Sdk/ndk/30.0.14904198"

CRATE_CC_NO_DEFAULTS = "1"
CMAKE_SYSTEM_VERSION = "24"
CMAKE_SYSTEM_NAME="Android"

AR = "~/Android/Sdk/ndk/30.0.14904198/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"

CC_aarch64_linux_android = "~/Android/Sdk/ndk/30.0.14904198/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang"
CXX_aarch64_linux_android = "~/Android/Sdk/ndk/30.0.14904198/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang++"
CFLAGS_aarch64_linux_android = "--target=aarch64-linux-android24 -march=armv8-a"
CXXFLAGS_aarch64_linux_android = "--target=aarch64-linux-android24 -march=armv8-a"

CC_armv7_linux_androideabi = "~/Android/Sdk/ndk/30.0.14904198/toolchains/llvm/prebuilt/linux-x86_64/bin/armv7a-linux-androideabi24-clang"
CXX_armv7_linux_androideabi = "~/Android/Sdk/ndk/30.0.14904198/toolchains/llvm/prebuilt/linux-x86_64/bin/armv7a-linux-androideabi24-clang++"
CFLAGS_armv7_linux_androideabi = "--target=armv7-linux-androideabi24 -march=armv7-a -mfloat-abi=softfp -mfpu=vfpv3-d16"
CXXFLAGS_armv7_linux_androideabi = "--target=armv7-linux-androideabi24 -march=armv7-a -mfloat-abi=softfp -mfpu=vfpv3-d16"
CMAKE_ANDROID_ARCH_ABI_armv7_linux_androideabi="armeabi-v7a"

```

run with (does not works for emulator)
```bash
CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang" CMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake" CMAKE_SYSTEM_NAME="Android" CMAKE_SYSTEM_VERSION="24" CMAKE_ANDROID_ARCH_ABI="arm64-v8a" CMAKE_ANDROID_NDK="$ANDROID_NDK_HOME" CC="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang" CXX="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang++" AR="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar" ANDROID_LLVM_TRIPLE="aarch64-linux-android" CFLAGS="--target=aarch64-linux-android24 -march=armv8-a" CXXFLAGS="--target=aarch64-linux-android24 -march=armv8-a" CMAKE_ASM_FLAGS="--target=aarch64-linux-android24" CRATE_CC_NO_DEFAULTS=1 CRATE_CC_NO_DEFAULTS=1 yarn tauri android dev
```

build seperately 
```bash
BINDGEN_EXTRA_CLANG_ARGS_armv7_linux_androideabi="--target=armv7-linux-androideabi24 -I $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/include" \
yarn tauri android build --aab --target armv7
yarn tauri android build --aab --target aarch64
BINDGEN_EXTRA_CLANG_ARGS_armv7_linux_androideabi="--target=armv7-linux-androideabi24 -I $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/include" \
yarn tauri android build --aab --target armv7 --target aarch64
```

## Usage Guide

Integrating the API on your frontend is straightforward. Below is the typical lifecycle of a transcription workflow:
### 1. Initializing the Model

Before executing any transcription tasks, you must load a local Whisper model file (.bin).

```typescript
import { initialize } from "./services/whisper";

async function setupWhisper() {
  const response = await initialize({
    modelPath: "/path/to/ggml-base.bin"
  });

  if (response.status) {
    console.log("Whisper model initialized successfully:", response.message);
  } else {
    console.error("Failed to initialize model:", response.message);
  }
}
```
### 2. Transcribing from an Audio File (Recommended)

To avoid UI blocking and optimize performance when dealing with large audio recordings, send the absolute file path directly to the backend.

```typescript
import { transcribeFromFile } from "./services/whisper";

async function processAudioFile() {
  const result = await transcribeFromFile({
    audioPath: "/path/to/audio.mp3",
    language: "en",       // ISO language code
    beamSize: 5,          // Higher beam size for better quality
    patience: 1.0
  });

  if (result.error) {
    console.error("Transcription error:", result.error);
  } else {
    console.log("Transcribed text:", result.text);
  }
}
```
### 3. Transcribing Raw Audio (In-Memory)

If you are capturing live audio feeds from a microphone and already have a sequence of `f32 PCM` samples, you can use the following method:

```typescript
import { transcribe } from "./services/whisper";

async function processRawAudio(pcmSamples: number[]) {
  const result = await transcribe({
    audioData: pcmSamples,
    language: "en"
  });

  if (!result.error) {
    console.log("Result:", result.text);
  }
}
```
### 4. Releasing Memory Resources

When the transcription tasks are completed or the user leaves the transcription view, trigger the `release` function to free up system resources (RAM/VRAM) and avoid memory leaks:

```typescript
import { release } from "./services/whisper";

async function cleanup() {
  const response = await release();
  if (response.status) {
    console.log("Model successfully released from memory.");
  }
}
```
## Data Structures (Type Definitions)

The module exposes the following TypeScript structural interfaces:

    `InitializeRequest`: Encapsulates the target model path specification.

    `TranscriptionRequest`: Contains the structural raw numerical audio array along with optional hyperparameters.

    `TranscriptionFileRequest`: Contains the local audio file path target along with optional hyperparameters.

    `TranscriptionResponse`: Carries either the resulting `text` output or the detailed `error` string payload.

    `GenericResponse`: Provides basic operation outcomes utilizing boolean `status` flags and detailed contextual `message` logs.