use crate::{
    ASR_SAMPLE_RATE, AsrBackendInfo, AsrCapabilities, Result, SpeechError, SpeechSegment,
    SpeechToTextEngine, VoiceActivityGate,
};
use engine_protocol::AudioSource;
use std::{
    ffi::{CStr, CString, c_char},
    sync::OnceLock,
};

unsafe extern "C" {
    fn rimv_apple_speech_available(locale_id: *const c_char) -> bool;
    fn rimv_apple_speech_supported_locales(output: *mut c_char, output_capacity: usize) -> i32;
    fn rimv_apple_speech_authorized() -> bool;
    fn rimv_apple_speech_prepare(
        locale_id: *const c_char,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn rimv_apple_speech_transcribe(
        locale_id: *const c_char,
        samples: *const f32,
        sample_count: usize,
        wait_for_final: bool,
        output: *mut c_char,
        output_capacity: usize,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
}

pub(crate) struct AppleSpeechEngine {
    locale_id: CString,
}

/// Apple Native does not depend on the separately managed Silero model. This
/// small energy gate only segments the stream; Apple Speech performs ASR.
#[derive(Default)]
pub(crate) struct AppleSpeechVad {
    trailing_silence_samples: usize,
    has_speech: bool,
}

const APPLE_SPEECH_ENERGY_THRESHOLD: f32 = 0.0015;
const APPLE_SPEECH_HANGOVER_MS: usize = 800;

impl VoiceActivityGate for AppleSpeechVad {
    fn is_speech(&mut self, samples: &[f32]) -> Result<bool> {
        let energy =
            samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len().max(1) as f32;
        if energy.sqrt() >= APPLE_SPEECH_ENERGY_THRESHOLD {
            self.trailing_silence_samples = 0;
            self.has_speech = true;
            return Ok(true);
        }
        if !self.has_speech {
            return Ok(false);
        }

        // Match the shared VAD end-silence interval. The energy gate has no
        // model-level hangover, so short pauses must not split Apple Speech
        // requests into word-sized fragments.
        self.trailing_silence_samples = self.trailing_silence_samples.saturating_add(samples.len());
        let hangover_samples = (ASR_SAMPLE_RATE as usize * APPLE_SPEECH_HANGOVER_MS) / 1000;
        if self.trailing_silence_samples < hangover_samples {
            Ok(true)
        } else {
            self.has_speech = false;
            Ok(false)
        }
    }

    fn flush(&mut self) -> Result<()> {
        self.trailing_silence_samples = 0;
        self.has_speech = false;
        Ok(())
    }
}

impl AppleSpeechEngine {
    pub(crate) fn new(locale: &str) -> Result<Self> {
        let resolved = resolve_locale(locale).ok_or_else(|| {
            let catalog = locale_catalog();
            let canonical = locale.replace('_', "-");
            let is_supported = catalog
                .supported
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(&canonical));
            let supports_on_device = catalog
                .supports_on_device
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(&canonical));
            let detail = if locale == "auto" || locale == "system" {
                "no preferred Apple Speech locale is currently usable on-device".to_owned()
            } else if !is_supported {
                format!("Apple Speech does not support locale {locale}")
            } else if !supports_on_device {
                format!("Apple Speech supports {locale}, but not on-device recognition")
            } else {
                format!("Apple Speech on-device assets are currently unavailable for {locale}")
            };
            SpeechError::ModelLoad(detail)
        })?;
        let locale_id = CString::new(resolved.as_str())
            .map_err(|error| SpeechError::ModelLoad(error.to_string()))?;
        let mut error = vec![0_i8; 2 * 1024];
        // SAFETY: locale_id and the writable error buffer remain alive for the
        // synchronous availability and permission check.
        let status = unsafe {
            rimv_apple_speech_prepare(locale_id.as_ptr(), error.as_mut_ptr(), error.len())
        };
        if status != 0 {
            let detail = unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy();
            return Err(SpeechError::ModelLoad(format!(
                "Apple Speech unavailable: {detail}"
            )));
        }
        tracing::debug!(
            engine = "native_apple",
            requested_locale = locale,
            resolved_locale = resolved,
            on_device = true,
            "Apple Speech recognizer prepared"
        );
        Ok(Self { locale_id })
    }

    fn recognize(
        &self,
        samples: &[f32],
        offset_ms: u64,
        wait_for_final: bool,
    ) -> Result<Vec<SpeechSegment>> {
        if samples.is_empty() {
            return Ok(Vec::new());
        }
        let mut output = vec![0_i8; 16 * 1024];
        let mut error = vec![0_i8; 2 * 1024];
        // SAFETY: all buffers remain valid for the synchronous call; output
        // and error capacities include their trailing NUL byte.
        let status = unsafe {
            rimv_apple_speech_transcribe(
                self.locale_id.as_ptr(),
                samples.as_ptr(),
                samples.len(),
                wait_for_final,
                output.as_mut_ptr(),
                output.len(),
                error.as_mut_ptr(),
                error.len(),
            )
        };
        if status != 0 {
            // SAFETY: native code writes a NUL-terminated message into the
            // provided buffer on every error return.
            let detail = unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy();
            return Err(SpeechError::Inference(format!(
                "Apple Speech error {status}: {detail}"
            )));
        }
        // SAFETY: successful native calls NUL-terminate output.
        let text = unsafe { CStr::from_ptr(output.as_ptr()) }
            .to_string_lossy()
            .trim()
            .to_owned();
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let duration_ms = samples.len() as u64 * 1000 / ASR_SAMPLE_RATE as u64;
        Ok(vec![SpeechSegment {
            source: AudioSource::Microphone,
            start_ms: offset_ms,
            end_ms: offset_ms.saturating_add(duration_ms),
            text,
        }])
    }
}

impl SpeechToTextEngine for AppleSpeechEngine {
    fn info(&self) -> AsrBackendInfo {
        AsrBackendInfo {
            backend_id: "native_apple".into(),
            backend_name: "Apple Speech".into(),
            model_id: "apple-speech".into(),
            model_name: "Apple Speech (on-device)".into(),
            capabilities: AsrCapabilities {
                supports_incremental_audio: true,
                supports_partial_results: true,
                supports_word_timestamps: false,
                supports_language_detection: false,
                supports_true_streaming: false,
            },
        }
    }

    fn transcribe(&mut self, audio: &[f32], offset_ms: u64) -> Result<Vec<SpeechSegment>> {
        self.recognize(audio, offset_ms, true)
    }

    fn transcribe_partial(&mut self, audio: &[f32], offset_ms: u64) -> Result<Vec<SpeechSegment>> {
        self.recognize(audio, offset_ms, false)
    }

    fn transcribe_final(&mut self, audio: &[f32], offset_ms: u64) -> Result<Vec<SpeechSegment>> {
        self.recognize(audio, offset_ms, true)
    }
}

pub(crate) fn available(locale: &str) -> bool {
    let Some(locale_id) = resolve_locale(locale) else {
        return false;
    };
    let Ok(locale_id) = CString::new(locale_id) else {
        return false;
    };
    // SAFETY: valid locale string, synchronously inspected by the native API.
    unsafe { rimv_apple_speech_available(locale_id.as_ptr()) }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// Runtime Apple Speech locale capabilities reported for the current Mac.
pub struct AppleSpeechLocaleCatalog {
    /// Locales advertised by `SFSpeechRecognizer.supportedLocales()`.
    pub supported: Vec<String>,
    /// Supported locales whose recognizer reports `supportsOnDeviceRecognition`.
    pub supports_on_device: Vec<String>,
    /// On-device locales that also report `isAvailable` now.
    pub usable_on_device: Vec<String>,
    /// The first system preferred locale that is currently usable on-device.
    pub preferred_locale: Option<String>,
}

static LOCALE_CATALOG: OnceLock<AppleSpeechLocaleCatalog> = OnceLock::new();

pub(crate) fn locale_catalog() -> AppleSpeechLocaleCatalog {
    LOCALE_CATALOG.get_or_init(read_locale_catalog).clone()
}

fn read_locale_catalog() -> AppleSpeechLocaleCatalog {
    let mut output = vec![0_i8; 64 * 1024];
    // SAFETY: native code writes a NUL-terminated JSON catalog within this
    // fixed buffer or returns an error without exposing a partial result.
    let count = unsafe { rimv_apple_speech_supported_locales(output.as_mut_ptr(), output.len()) };
    if count < 0 {
        return AppleSpeechLocaleCatalog::default();
    }
    // SAFETY: successful native calls NUL-terminate the JSON output.
    let value = unsafe { CStr::from_ptr(output.as_ptr()) }.to_string_lossy();
    let catalog: serde_json::Value = match serde_json::from_str(&value) {
        Ok(catalog) => catalog,
        Err(_) => return AppleSpeechLocaleCatalog::default(),
    };
    let parse = |key: &str| {
        catalog[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(|locale| locale.replace('_', "-"))
            .collect()
    };
    AppleSpeechLocaleCatalog {
        supported: parse("supported"),
        supports_on_device: parse("on_device"),
        usable_on_device: parse("usable_on_device"),
        preferred_locale: catalog["preferred_locale"]
            .as_str()
            .map(|locale| locale.replace('_', "-")),
    }
}

pub fn apple_speech_locale_catalog() -> AppleSpeechLocaleCatalog {
    locale_catalog()
}

pub fn default_locale() -> Option<String> {
    locale_catalog().preferred_locale
}

fn resolve_locale(locale: &str) -> Option<String> {
    if locale == "auto" || locale == "system" {
        return default_locale();
    }
    if locale.contains('-') || locale.contains('_') {
        let canonical = locale.replace('_', "-");
        locale_catalog()
            .usable_on_device
            .iter()
            .find(|candidate| candidate.eq_ignore_ascii_case(&canonical))
            .cloned()
    } else {
        apple_locale(locale).map(str::to_owned)
    }
}

pub fn apple_locale(language: &str) -> Option<&'static str> {
    Some(match language {
        "auto" | "en" | "en-US" => "en-US",
        "ar" => "ar-SA",
        "de" => "de-DE",
        "es" => "es-ES",
        "fr" => "fr-FR",
        "hi" => "hi-IN",
        "it" => "it-IT",
        "ja" => "ja-JP",
        "nl" => "nl-NL",
        "pt" => "pt-BR",
        "ru" => "ru-RU",
        "zh" => "zh-CN",
        _ => return None,
    })
}

pub fn apple_speech_available(locale: &str) -> bool {
    available(locale)
}

/// Returns whether macOS has granted Speech Recognition permission to this process.
pub fn apple_speech_authorized() -> bool {
    // SAFETY: the function reads the process authorization state and takes no pointers.
    unsafe { rimv_apple_speech_authorized() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_vad_keeps_short_pauses_inside_an_utterance() {
        let mut vad = AppleSpeechVad::default();
        let speech = vec![0.002; 1_600];
        let silence = vec![0.0005; 1_600];
        assert!(vad.is_speech(&speech).unwrap());
        for _ in 0..7 {
            assert!(vad.is_speech(&silence).unwrap());
        }
        assert!(!vad.is_speech(&silence).unwrap());
        assert!(!vad.is_speech(&silence).unwrap());
        assert!(vad.is_speech(&speech).unwrap());
    }
}
