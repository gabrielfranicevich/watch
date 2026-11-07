cmake_minimum_required(VERSION 3.22.1)
project(watch_jni LANGUAGES CXX)

if(ANDROID_ABI STREQUAL "arm64-v8a")
  set(RUST_TARGET "aarch64-linux-android")
elseif(ANDROID_ABI STREQUAL "x86_64")
  set(RUST_TARGET "x86_64-linux-android")
else()
  message(FATAL_ERROR "Unsupported Android ABI: ${ANDROID_ABI}")
endif()

add_library(watch_rust STATIC IMPORTED)
set_target_properties(watch_rust PROPERTIES
  IMPORTED_LOCATION "${CMAKE_CURRENT_SOURCE_DIR}/../../../../target/${RUST_TARGET}/release/libani_cli.a"
)

add_library(watch_jni SHARED watch_jni.cpp)
target_include_directories(watch_jni PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/../../../../include")
target_compile_features(watch_jni PRIVATE cxx_std_17)
target_link_libraries(watch_jni PRIVATE watch_rust log android dl m z)
