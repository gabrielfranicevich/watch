#include <jni.h>
#include <cstdint>
#include "watch_android_bridge.h"

static const char *read_string(JNIEnv *env, jstring value) {
  if (value == nullptr) return nullptr;
  return env->GetStringUTFChars(value, nullptr);
}

static void release_string(JNIEnv *env, jstring value, const char *chars) {
  if (value != nullptr && chars != nullptr) {
    env->ReleaseStringUTFChars(value, chars);
  }
}

static jstring result_string(JNIEnv *env, char *value) {
  if (value == nullptr) return env->NewStringUTF("{\"error\":\"native bridge returned no result\"}");
  jstring result = env->NewStringUTF(value);
  watch_string_free(value);
  return result;
}

extern "C" JNIEXPORT jlong JNICALL
Java_com_gabrielfranicevich_watch_NativeWatch_nativeCreate(
  JNIEnv *env, jobject, jstring history_path
) {
  const char *path = read_string(env, history_path);
  const uint64_t handle = watch_application_create(path);
  release_string(env, history_path, path);
  return static_cast<jlong>(handle);
}

extern "C" JNIEXPORT void JNICALL
Java_com_gabrielfranicevich_watch_NativeWatch_nativeDestroy(
  JNIEnv *, jobject, jlong handle
) {
  watch_application_destroy(static_cast<uint64_t>(handle));
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_gabrielfranicevich_watch_NativeWatch_nativeSearch(
  JNIEnv *env, jobject, jlong handle, jstring query, jstring language
) {
  const char *q = read_string(env, query);
  const char *lang = read_string(env, language);
  char *result = watch_search(static_cast<uint64_t>(handle), q, lang);
  release_string(env, query, q);
  release_string(env, language, lang);
  return result_string(env, result);
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_gabrielfranicevich_watch_NativeWatch_nativeEpisodes(
  JNIEnv *env, jobject, jlong handle, jstring provider, jstring anime_id,
  jstring title, jstring url
) {
  const char *p = read_string(env, provider);
  const char *id = read_string(env, anime_id);
  const char *t = read_string(env, title);
  const char *u = read_string(env, url);
  char *result = watch_episodes(static_cast<uint64_t>(handle), p, id, t, u);
  release_string(env, provider, p);
  release_string(env, anime_id, id);
  release_string(env, title, t);
  release_string(env, url, u);
  return result_string(env, result);
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_gabrielfranicevich_watch_NativeWatch_nativeResolve(
  JNIEnv *env, jobject, jlong handle, jstring provider, jstring number,
  jstring title, jstring url, jboolean dubbed, jstring quality
) {
  const char *p = read_string(env, provider);
  const char *n = read_string(env, number);
  const char *t = read_string(env, title);
  const char *u = read_string(env, url);
  const char *q = read_string(env, quality);
  char *result = watch_resolve(
    static_cast<uint64_t>(handle), p, n, t, u, dubbed == JNI_TRUE, q
  );
  release_string(env, provider, p);
  release_string(env, number, n);
  release_string(env, title, t);
  release_string(env, url, u);
  release_string(env, quality, q);
  return result_string(env, result);
}
