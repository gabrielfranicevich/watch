use std::ffi::CString;

use crate::android_bridge::{
  watch_application_create, watch_application_destroy, watch_search, watch_string_free,
};

#[test]
fn bridge_rejects_invalid_handles_with_json_errors() {
  let query = CString::new("frieren").unwrap();
  let result = unsafe { watch_search(999_999, query.as_ptr(), std::ptr::null()) };
  assert!(!result.is_null());
  let json = unsafe { std::ffi::CStr::from_ptr(result) }
    .to_str()
    .unwrap();
  let value: serde_json::Value = serde_json::from_str(json).unwrap();
  assert!(value["error"].as_str().unwrap().contains("handle"));
  unsafe { watch_string_free(result) };
}

#[test]
fn bridge_creates_and_destroys_application_handles() {
  let history = CString::new("").unwrap();
  let handle = unsafe { watch_application_create(history.as_ptr()) };
  assert_ne!(handle, 0);
  watch_application_destroy(handle);

  let query = CString::new("frieren").unwrap();
  let result = unsafe { watch_search(handle, query.as_ptr(), std::ptr::null()) };
  let json = unsafe { std::ffi::CStr::from_ptr(result) }
    .to_str()
    .unwrap();
  assert!(json.contains("invalid or has been closed"));
  unsafe { watch_string_free(result) };
}

#[test]
fn bridge_validates_language_before_network_access() {
  let history = CString::new("").unwrap();
  let handle = unsafe { watch_application_create(history.as_ptr()) };
  let query = CString::new("frieren").unwrap();
  let language = CString::new("fr").unwrap();
  let result = unsafe { watch_search(handle, query.as_ptr(), language.as_ptr()) };
  let json = unsafe { std::ffi::CStr::from_ptr(result) }
    .to_str()
    .unwrap();
  assert!(json.contains("unsupported language"));
  unsafe { watch_string_free(result) };
  watch_application_destroy(handle);
}

#[test]
fn bridge_delegates_empty_query_validation_to_application_service() {
  let history = CString::new("").unwrap();
  let handle = unsafe { watch_application_create(history.as_ptr()) };
  let query = CString::new("  ").unwrap();
  let result = unsafe { watch_search(handle, query.as_ptr(), std::ptr::null()) };
  let json = unsafe { std::ffi::CStr::from_ptr(result) }
    .to_str()
    .unwrap();
  assert!(json.contains("search query must not be empty"));
  unsafe { watch_string_free(result) };
  watch_application_destroy(handle);
}
