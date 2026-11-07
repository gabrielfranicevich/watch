# Native Android application

The native Android client lives in `app/`. It uses Kotlin, Jetpack Compose, Media3, and a Rust static library. The Rust application services remain the only implementation of provider search, episode extraction, and stream resolution.

## Architecture

- `src/application_services.rs`: shared Rust use cases.
- `src/android_bridge.rs`: C ABI returning JSON DTOs and owning application handles.
- `include/watch_android_bridge.h`: C ABI contract.
- `app/src/main/cpp/watch_jni.cpp`: thin JNI adapter; it does not implement application logic.
- `app/src/main/java/`: Compose UI and the Kotlin JNI wrapper.
- Media3 handles HLS/progressive playback, Referer headers, and supported external subtitle formats. Separate audio is merged as a second media source when the resolver supplies an audio URL.

## Build prerequisites

Install JDK 17, Android SDK Platform 35, Android NDK 27.2.12479018, CMake 3.22.1, Gradle 8.10.2, Rust stable, and the Android Rust targets:

```sh
rustup target add aarch64-linux-android x86_64-linux-android
```

Set `ANDROID_HOME` and `ANDROID_NDK_HOME`, then build from the repository root:

```sh
gradle :app:assembleDebug
```

The Gradle build compiles the Rust static library for ARM64 physical devices and x86_64 emulators before CMake links the JNI shared library. The Android package currently targets API 26 or newer.

## Automated validation

Run Rust bridge tests and Android instrumentation tests:

```sh
cargo test --locked --all-targets
gradle :app:connectedDebugAndroidTest
```

Instrumentation tests verify that the installed APK starts, loads its packaged JNI/Rust libraries, creates and destroys a Rust application handle, and returns structured validation errors. They do not prove that a live third-party provider is reachable or that every media host/player combination works.

## Physical-device acceptance

A physical-device run is still required before declaring native playback validated. Install the debug APK on an ARM64 device and record:

- Device model, Android version/API, ABI, and security patch.
- APK version/commit, provider, selected title/episode, and player state.
- Whether search, source aggregation, episode selection, and history work.
- HLS and progressive MP4 playback, including a stream requiring a Referer.
- Subtitle display for VTT and SRT sources when supplied.
- Separate audio behavior when the resolver supplies an audio URL.
- Pause/resume, seek, orientation changes, background/foreground transitions, and repeated playback with different stream headers.
- Failure logs from `adb logcat` with personal data and signed media URLs redacted.

For a transport-only baseline, the public HLS test stream used by the Termux guide can be opened independently. A successful baseline does not validate provider resolution. Do not mark physical playback acceptance complete based only on an emulator build or a successful Rust cross-compilation.
