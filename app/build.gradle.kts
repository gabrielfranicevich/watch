plugins {
  id("com.android.application")
  id("org.jetbrains.kotlin.android")
  id("org.jetbrains.kotlin.plugin.compose")
}

android {
  namespace = "com.gabrielfranicevich.watch"
  compileSdk = 35

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

    ndk {
      abiFilters += listOf("arm64-v8a")
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
  debugImplementation("androidx.compose.ui:ui-tooling")
}

val ndkVersionForRust = "27.2.12479018"
val rustTarget = "aarch64-linux-android"
val rustLinker = providers.environmentVariable("ANDROID_NDK_HOME")
  .orElse(providers.environmentVariable("ANDROID_NDK_ROOT"))
  .orElse(providers.environmentVariable("ANDROID_HOME").map { "$it/ndk/$ndkVersionForRust" })
  .map { "$it/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang" }

tasks.register<Exec>("buildRustAndroid") {
  workingDir = rootProject.projectDir.parentFile
  commandLine("cargo", "build", "--locked", "--release", "--lib", "--target", rustTarget)
  environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", rustLinker.get())
  environment("CC_aarch64_linux_android", rustLinker.get())
}

tasks.configureEach {
  if (name.startsWith("configureCMake") || name.startsWith("buildCMake")) {
    dependsOn("buildRustAndroid")
  }
}
