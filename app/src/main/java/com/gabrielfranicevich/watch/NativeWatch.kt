package com.gabrielfranicevich.watch

internal class NativeWatch(historyPath: String) : AutoCloseable {
  private var handle: Long = nativeCreate(historyPath)

  init {
    check(handle != 0L) { "Could not initialize the Rust application" }
  }

  fun search(query: String, language: String? = null): String =
    nativeSearch(requireHandle(), query, language.orEmpty())

  fun episodes(provider: String, animeId: String, title: String, url: String): String =
    nativeEpisodes(requireHandle(), provider, animeId, title, url)

  fun resolve(
    provider: String,
    number: String,
    title: String?,
    url: String,
    dubbed: Boolean = false,
    quality: String = "best"
  ): String = nativeResolve(requireHandle(), provider, number, title.orEmpty(), url, dubbed, quality)

  override fun close() {
    val current = handle
    if (current != 0L) {
      nativeDestroy(current)
      handle = 0L
    }
  }

  private fun requireHandle(): Long {
    check(handle != 0L) { "Rust application is closed" }
    return handle
  }

  private external fun nativeCreate(historyPath: String): Long
  private external fun nativeDestroy(handle: Long)
  private external fun nativeSearch(handle: Long, query: String, language: String): String
  private external fun nativeEpisodes(
    handle: Long,
    provider: String,
    animeId: String,
    title: String,
    url: String
  ): String
  private external fun nativeResolve(
    handle: Long,
    provider: String,
    number: String,
    title: String,
    url: String,
    dubbed: Boolean,
    quality: String
  ): String

  companion object {
    init {
      System.loadLibrary("watch_jni")
    }
  }
}
