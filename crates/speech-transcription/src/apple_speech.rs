use crate::{
    ASR_SAMPLE_RATE, AsrBackendInfo, AsrCapabilities, Result, SpeechError, SpeechSegment,
    SpeechToTextEngine, VoiceActivityGate,
};
use engine_protocol::AudioSource;
use std::ffi::{CStr, CString, c_char};

unsafe extern "C" {
    fn rimv_apple_speech_available(locale_id: *const c_char) -> bool;
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
pub(crate) struct AppleSpeechVad;

impl VoiceActivityGate for AppleSpeechVad {
    fn is_speech(&mut self, samples: &[f32]) -> Result<bool> {
        let energy =
            samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len().max(1) as f32;
        Ok(energy.sqrt() >= 0.008)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl AppleSpeechEngine {
    pub(crate) fn new(locale: &str) -> Result<Self> {
        let locale_id = apple_locale(locale).ok_or_else(|| {
            SpeechError::ModelLoad(format!("unsupported Apple Speech locale: {locale}"))
        })?;
        let locale_id =
            CString::new(locale_id).map_err(|error| SpeechError::ModelLoad(error.to_string()))?;
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
    let Some(locale_id) = apple_locale(locale) else {
        return false;
    };
    let Ok(locale_id) = CString::new(locale_id) else {
        return false;
    };
    // SAFETY: valid locale string, synchronously inspected by the native API.
    unsafe { rimv_apple_speech_available(locale_id.as_ptr()) }
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
