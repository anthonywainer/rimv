use rimv_core_ffi::{
    API_VERSION, RimvEngine, rimv_api_version, rimv_engine_create, rimv_engine_destroy,
    rimv_engine_request, rimv_string_free,
};
use serde_json::{Value, json};
use std::{
    ffi::{CStr, CString, c_char},
    path::Path,
};

struct OwnedResponse(*mut c_char);
impl OwnedResponse {
    fn json(self) -> Value {
        // SAFETY: this value is constructed only from an owned string returned by the FFI.
        let bytes = unsafe { CStr::from_ptr(self.0) }.to_bytes().to_vec();
        serde_json::from_slice(&bytes).unwrap()
    }
}
impl Drop for OwnedResponse {
    fn drop(&mut self) {
        // SAFETY: the wrapped pointer is a non-null response returned by this library.
        unsafe { rimv_string_free(self.0) };
    }
}

struct Engine(*mut RimvEngine);
impl Engine {
    fn request(&mut self, request: &str) -> Value {
        let request = CString::new(request).unwrap();
        // SAFETY: this handle remains uniquely owned and request is NUL-terminated UTF-8.
        OwnedResponse(unsafe { rimv_engine_request(self.0, request.as_ptr()) }).json()
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        // SAFETY: this wrapper uniquely owns a handle created by rimv_engine_create.
        unsafe { rimv_engine_destroy(self.0) };
    }
}

fn create_engine(root: &Path) -> (Engine, Value) {
    let config = CString::new(
        json!({
            "recordings_directory": root.join("recordings"),
            "models_directory": root.join("models"),
        })
        .to_string(),
    )
    .unwrap();
    let mut handle = std::ptr::null_mut();
    // SAFETY: input is valid UTF-8 and the out pointer is writable.
    let response =
        OwnedResponse(unsafe { rimv_engine_create(config.as_ptr(), &mut handle) }).json();
    assert!(!handle.is_null());
    (Engine(handle), response)
}

#[test]
fn c_abi_version_lifecycle_and_structured_errors_match_the_header_contract() {
    assert_eq!(rimv_api_version(), API_VERSION);
    let root = tempfile::tempdir().unwrap();
    let (mut engine, created) = create_engine(root.path());
    assert_eq!(created["ok"], true);
    assert_eq!(created["result"]["api_version"], API_VERSION);

    let state = engine.request(r#"{"type":"get_state"}"#);
    assert_eq!(state["ok"], true);
    assert_eq!(state["result"]["status"], "idle");

    let initial_event = engine.request(r#"{"type":"poll_event","timeout_ms":500}"#);
    assert_eq!(initial_event["ok"], true);
    assert_eq!(initial_event["result"]["type"], "snapshot");

    let models = engine.request(r#"{"type":"get_models"}"#);
    assert_eq!(models["ok"], true);
    for model in models["result"].as_array().unwrap() {
        if model["descriptor"]["backend"] == "whisper" && cfg!(target_os = "windows") {
            assert_eq!(model["state"], "unsupported");
        }
    }

    let changed = engine
        .request(r#"{"type":"send","command":{"type":"set_microphone_enabled","enabled":false}}"#);
    assert_eq!(changed["ok"], true);
    let changed_event = engine.request(r#"{"type":"poll_event","timeout_ms":500}"#);
    assert_eq!(changed_event["ok"], true);
    assert_eq!(changed_event["result"]["type"], "snapshot");
    assert_eq!(
        changed_event["result"]["snapshot"]["microphone"]["enabled"],
        false
    );

    for request in ["not-json", r#"{"type":"unknown_operation"}"#] {
        let error = engine.request(request);
        assert_eq!(error["ok"], false);
        assert_eq!(error["error"]["code"], "request_failed");
        assert!(
            error["error"]["message"]
                .as_str()
                .is_some_and(|message| !message.is_empty())
        );
    }
    // SAFETY: the live handle is valid; null input is a documented error case.
    let null_request =
        OwnedResponse(unsafe { rimv_engine_request(engine.0, std::ptr::null()) }).json();
    assert_eq!(null_request["ok"], false);
    // SAFETY: null is explicitly accepted by string_free.
    unsafe { rimv_string_free(std::ptr::null_mut()) };

    let invalid_export =
        engine.request(r#"{"type":"export","session_id":"not-a-session","format":"html"}"#);
    assert_eq!(invalid_export["ok"], false);
    assert!(
        invalid_export["error"]["message"]
            .as_str()
            .unwrap()
            .contains("format")
    );
    assert_eq!(engine.request(r#"{"type":"shutdown"}"#)["ok"], true);
}

#[test]
fn null_handle_returns_owned_error_envelope_without_dereferencing_request() {
    // SAFETY: null is explicitly accepted by the API and the returned envelope is caller-owned.
    let response =
        OwnedResponse(unsafe { rimv_engine_request(std::ptr::null_mut(), std::ptr::null()) })
            .json();
    assert_eq!(response["ok"], false);
}

#[test]
fn invalid_configuration_returns_an_error_without_an_engine_handle() {
    let invalid = CString::new("{}").unwrap();
    let mut handle = std::ptr::null_mut();
    // SAFETY: config is valid NUL-terminated UTF-8 and the out pointer is writable.
    let response =
        OwnedResponse(unsafe { rimv_engine_create(invalid.as_ptr(), &mut handle) }).json();
    assert_eq!(response["ok"], false);
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("recordings_directory")
    );
    assert!(handle.is_null());
}
