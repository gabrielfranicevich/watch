plugins {
  id("com.android.application")
  id("org.jetbrains.kotlin.android")
  id("org.jetbrains.kotlin.plugin.compose")
}

android {
  namespace = "com.gabrielfranicevich.watch"
  compileSdk = 35
  ndkVersion = "27.2.12479018"

  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
  }

  kotlinOptions {
    jvmTarget = "17"
  }

  defaultConfig {
    applicationId = "com.gabrielfranicevich.watch"
    minSdk = 26
    targetSdk = 35
    versionCode = 1
    versionName = "0.1.0"
    testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

    ndk {
      abiFilters += listOf("arm64-v8a", "x86_64")
    }

    externalNativeBuild {
      cmake {
        cppFlags += "-std=c++17"
      }
    }
  }

  buildFeatures {
    compose = true
  }

  externalNativeBuild {
    cmake {
      path = file("src/main/cpp/CMakeLists.txt")
      version = "3.22.1"
    }
  }

  packaging {
    jniLibs {
      useLegacyPackaging = false
    }
  }
}

dependencies {
  implementation(platform("androidx.compose:compose-bom:2024.12.01"))
  implementation("androidx.activity:activity-compose:1.10.0")
  implementation("androidx.compose.material3:material3")
  implementation("androidx.compose.ui:ui")
  implementation("androidx.compose.ui:ui-tooling-preview")
  implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.7")
  implementation("androidx.media3:media3-exoplayer:1.5.1")
  implementation("androidx.media3:media3-exoplayer-hls:1.5.1")
  implementation("androidx.media3:media3-ui:1.5.1")
  debugImplementation("androidx.compose.ui:ui-tooling")
  androidTestImplementation("androidx.test.ext:junit:1.2.1")
  androidTestImplementation("androidx.test:runner:1.6.2")
  androidTestImplementation("androidx.test:core:1.6.1")
}

val ndkVersionForRust = "27.2.12479018"
val rustNdkRoot = providers.environmentVariable("ANDROID_NDK_HOME")
  .orElse(providers.environmentVariable("ANDROID_NDK_ROOT"))
  .orElse(providers.environmentVariable("ANDROID_HOME").map { "$it/ndk/$ndkVersionForRust" })
val hostTag = when {
  System.getProperty("os.name").startsWith("Windows") -> "windows-x86_64"
  System.getProperty("os.name").startsWith("Mac") -> "darwin-x86_64"
  else -> "linux-x86_64"
}
val rustNdkBin = rustNdkRoot.map {
  "$it/toolchains/llvm/prebuilt/$hostTag/bin"
}

tasks.register<Exec>("buildRustAndroidArm64") {
  workingDir = rootProject.projectDir
  commandLine("cargo", "build", "--locked", "--release", "--lib", "--target", "aarch64-linux-android")
  environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", rustNdkBin.map { "$it/aarch64-linux-android24-clang" }.get())
  environment("CC_aarch64_linux_android", rustNdkBin.map { "$it/aarch64-linux-android24-clang" }.get())
  environment("AR_aarch64_linux_android", rustNdkBin.map { "$it/llvm-ar" }.get())
}

tasks.register<Exec>("buildRustAndroidX64") {
  workingDir = rootProject.projectDir
  commandLine("cargo", "build", "--locked", "--release", "--lib", "--target", "x86_64-linux-android")
  environment("CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER", rustNdkBin.map { "$it/x86_64-linux-android24-clang" }.get())
  environment("CC_x86_64_linux_android", rustNdkBin.map { "$it/x86_64-linux-android24-clang" }.get())
  environment("AR_x86_64_linux_android", rustNdkBin.map { "$it/llvm-ar" }.get())
}

tasks.register("buildRustAndroid") {
  dependsOn("buildRustAndroidArm64", "buildRustAndroidX64")
}

tasks.configureEach {
  if (name.startsWith("configureCMake") || name.startsWith("buildCMake")) {
    dependsOn("buildRustAndroid")
  }
}
