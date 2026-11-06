use std::{
  collections::HashMap,
  ffi::{CStr, CString, c_char},
  panic::{AssertUnwindSafe, catch_unwind},
  path::PathBuf,
  sync::{
    Mutex, OnceLock,
    atomic::{AtomicU64, Ordering},
  },
};

use crate::{
  application_services::{PlaybackRequest, SearchRequest, WatchApplication},
  domain::{anime::Anime, episode::Episode},
  providers::{
    catalog::ProviderAnime,
    site::{Language, ProviderId},
  },
};

static APPLICATIONS: OnceLock<Mutex<HashMap<u64, WatchApplication>>> = OnceLock::new();
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

fn applications() -> &'static Mutex<HashMap<u64, WatchApplication>> {
  APPLICATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn application(handle: u64) -> Result<WatchApplication, String> {
  let registry = applications()
    .lock()
    .map_err(|_| "application registry is unavailable".to_owned())?;
  registry
    .get(&handle)
    .cloned()
    .ok_or_else(|| "application handle is invalid or has been closed".to_owned())
}

unsafe fn input<'a>(value: *const c_char) -> Result<&'a str, String> {
  if value.is_null() {
    return Err("native string argument is null".to_owned());
  }
  unsafe { CStr::from_ptr(value) }
    .to_str()
    .map_err(|error| format!("native string is not UTF-8: {error}"))
}

fn output(value: serde_json::Value) -> *mut c_char {
  let encoded = value.to_string();
  CString::new(encoded)
    .map(CString::into_raw)
    .unwrap_or(std::ptr::null_mut())
}

fn error(message: impl Into<String>) -> *mut c_char {
  output(serde_json::json!({ "error": message.into() }))
}

fn guarded(operation: impl FnOnce() -> *mut c_char) -> *mut c_char {
  match catch_unwind(AssertUnwindSafe(operation)) {
    Ok(result) => result,
    Err(_) => error("native bridge operation panicked"),
  }
}

/// # Safety
/// `history_path` must be null or point to a readable, NUL-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn watch_application_create(history_path: *const c_char) -> u64 {
  let result = catch_unwind(AssertUnwindSafe(|| {
    let path = if history_path.is_null() {
      None
    } else {
      unsafe { input(history_path) }
        .ok()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    };
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut applications) = applications().lock() {
      applications.insert(handle, WatchApplication::new(path));
      handle
    } else {
      0
    }
  }));
  result.unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn watch_application_destroy(handle: u64) {
  if let Ok(mut applications) = applications().lock() {
    drop(applications.remove(&handle));
  }
}

/// # Safety
/// `query` must point to a readable, NUL-terminated UTF-8 string. `language` must be null or satisfy the same requirement.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn watch_search(
  handle: u64,
  query: *const c_char,
  language: *const c_char,
) -> *mut c_char {
  guarded(|| {
    let query = match unsafe { input(query) } {
      Ok(value) => value,
      Err(message) => return error(message),
    };
    let language = if language.is_null() {
      None
    } else {
      match unsafe { input(language) } {
        Ok("en") => Some(Language::English),
        Ok("es") => Some(Language::Spanish),
        Ok("") => None,
        Ok(value) => return error(format!("unsupported language '{value}'")),
        Err(message) => return error(message),
      }
    };
    let application = match application(handle) {
      Ok(value) => value,
      Err(message) => return error(message),
    };
    match application.search(&SearchRequest {
      query: query.to_owned(),
      language,
      audio: None,
      subtitles: None,
    }) {
      Ok(options) => output(serde_json::json!({
        "results": options.into_iter().map(|option| serde_json::json!({
          "label": option.label,
          "sources": option.sources.into_iter().map(|source| serde_json::json!({
            "provider": source.provider.key(),
            "provider_name": source.provider.name(),
            "id": source.anime.id,
            "title": source.anime.title,
            "url": source.anime.url
          })).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
      })),
      Err(error) => error_json(error.to_string()),
    }
  })
}

/// # Safety
/// All string pointers must point to readable, NUL-terminated UTF-8 strings for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn watch_episodes(
  handle: u64,
  provider: *const c_char,
  anime_id: *const c_char,
  anime_title: *const c_char,
  anime_url: *const c_char,
) -> *mut c_char {
  guarded(|| {
    let provider = match unsafe { input(provider) }.and_then(|value| {
      ProviderId::from_key(value).ok_or_else(|| format!("unknown provider '{value}'"))
    }) {
      Ok(value) => value,
      Err(message) => return error(message),
    };
    let anime_id = match unsafe { input(anime_id) } {
      Ok(value) => value,
      Err(message) => return error(message),
    };
    let anime_title = match unsafe { input(anime_title) } {
      Ok(value) => value,
      Err(message) => return error(message),
    };
    let anime_url = match unsafe { input(anime_url) } {
      Ok(value) => value,
      Err(message) => return error(message),
    };
    let application = match application(handle) {
      Ok(value) => value,
      Err(message) => return error(message),
    };
    let source = ProviderAnime {
      provider,
      anime: Anime::new(anime_id, anime_title, anime_url),
    };
    match application.episodes(&source) {
      Ok(episodes) => output(serde_json::json!({
        "episodes": episodes.into_iter().map(|episode| serde_json::json!({
          "number": episode.number,
          "title": episode.title,
          "url": episode.url
        })).collect::<Vec<_>>()
      })),
      Err(error) => error_json(error.to_string()),
    }
  })
}

/// # Safety
/// Required string pointers must point to readable, NUL-terminated UTF-8 strings. `episode_title` may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn watch_resolve(
  handle: u64,
  provider: *const c_char,
  episode_number: *const c_char,
  episode_title: *const c_char,
  episode_url: *const c_char,
  dubbed: bool,
  quality: *const c_char,
) -> *mut c_char {
  guarded(|| {
    let provider = match unsafe { input(provider) }.and_then(|value| {
      ProviderId::from_key(value).ok_or_else(|| format!("unknown provider '{value}'"))
    }) {
      Ok(value) => value,
      Err(message) => return error(message),
    };
    let episode_number = match unsafe { input(episode_number) } {
      Ok(value) => value,
      Err(message) => return error(message),
    };
    let episode_title = if episode_title.is_null() {
      None
    } else {
      match unsafe { input(episode_title) } {
        Ok(value) if !value.is_empty() => Some(value.to_owned()),
        Ok(_) => None,
        Err(message) => return error(message),
      }
    };
    let episode_url = match unsafe { input(episode_url) } {
      Ok(value) => value,
      Err(message) => return error(message),
    };
    let quality = match unsafe { input(quality) } {
      Ok(value) => value,
      Err(message) => return error(message),
    };
    let application = match application(handle) {
      Ok(value) => value,
      Err(message) => return error(message),
    };
    let request = PlaybackRequest {
      episode: Episode::new(episode_number, episode_title, episode_url),
      provider,
      dubbed,
      quality: quality.to_owned(),
    };
    match application.resolve_stream(&request) {
      Ok(media) => output(serde_json::json!({
        "stream": {
          "url": media.stream.url,
          "quality": media.stream.quality,
          "referer": media.stream.referer,
          "audio_url": media.stream.audio_url
        },
        "subtitle": media.subtitle.map(|subtitle| serde_json::json!({
          "url": subtitle.url,
          "language": subtitle.language
        })),
        "qualities": media.qualities.into_iter().map(|quality| serde_json::json!({
          "label": quality.label,
          "value": quality.value
        })).collect::<Vec<_>>()
      })),
      Err(error) => error_json(error.to_string()),
    }
  })
}

fn error_json(message: String) -> *mut c_char {
  error(message)
}

/// # Safety
/// `value` must be null or a pointer previously returned by this bridge and not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn watch_string_free(value: *mut c_char) {
  if !value.is_null() {
    unsafe { drop(CString::from_raw(value)) };
  }
}

#[cfg(test)]
#[path = "../tests/unit/android_bridge.rs"]
mod tests;
