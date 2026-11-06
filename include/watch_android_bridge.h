#ifndef WATCH_ANDROID_BRIDGE_H
#define WATCH_ANDROID_BRIDGE_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

uint64_t watch_application_create(const char *history_path);
void watch_application_destroy(uint64_t handle);
char *watch_search(uint64_t handle, const char *query, const char *language);
char *watch_episodes(
  uint64_t handle,
  const char *provider,
  const char *anime_id,
  const char *anime_title,
  const char *anime_url
);
char *watch_resolve(
  uint64_t handle,
  const char *provider,
  const char *episode_number,
  const char *episode_title,
  const char *episode_url,
  bool dubbed,
  const char *quality
);
void watch_string_free(char *value);

#ifdef __cplusplus
}
#endif

#endif
