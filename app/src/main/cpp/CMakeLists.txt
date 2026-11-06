cmake_minimum_required(VERSION 3.22.1)
project(watch_jni LANGUAGES CXX)

add_library(watch_rust STATIC IMPORTED)
set_target_properties(watch_rust PROPERTIES
  IMPORTED_LOCATION "${CMAKE_CURRENT_SOURCE_DIR}/../../../../target/aarch64-linux-android/release/libani_cli.a"
)

add_library(watch_jni SHARED watch_jni.cpp)
target_include_directories(watch_jni PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/../../../../include")
target_compile_features(watch_jni PRIVATE cxx_std_17)
target_link_libraries(watch_jni PRIVATE watch_rust log android dl m z)
