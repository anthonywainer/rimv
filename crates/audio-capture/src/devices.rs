use crate::backend_error;
use audio_core::{AudioError, Result};
use cpal::traits::{DeviceTrait, HostTrait};
/// CPAL 0.16 IDs are useful selectors derived from direction, name and duplicate
/// ordinal. Re-enumerate after hardware changes; they are not persistent OS UIDs.
#[derive(Debug, Clone)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}
#[derive(Debug, Clone)]
pub struct SupportedAudioConfig {
    pub channels: u16,
    pub min_sample_rate: u32,
    pub max_sample_rate: u32,
    /// Native device representation; delivered frames are always f32.
    pub sample_format: String,
}
pub(crate) fn devices(input: bool) -> Result<Vec<(AudioDeviceInfo, cpal::Device)>> {
    let host = cpal::default_host();
    let default = if input {
        host.default_input_device()
    } else {
        host.default_output_device()
    };
    let default_name = default.and_then(|d| d.name().ok());
    let iter = if input {
        host.input_devices()
    } else {
        host.output_devices()
    }
    .map_err(backend_error)?;
    let mut names = std::collections::HashMap::new();
    let mut result = Vec::new();
    for device in iter {
        let name = device.name().map_err(backend_error)?;
        let ordinal = names.entry(name.clone()).or_insert(0);
        let id = format!(
            "{}:{name}:{}",
            if input { "input" } else { "output" },
            *ordinal
        );
        let is_default = default_name.as_ref() == Some(&name) && *ordinal == 0;
        *ordinal += 1;
        result.push((
            AudioDeviceInfo {
                id,
                name,
                is_default,
            },
            device,
        ));
    }
    Ok(result)
}
pub fn list_microphones() -> Result<Vec<AudioDeviceInfo>> {
    Ok(devices(true)?.into_iter().map(|(i, _)| i).collect())
}
pub fn list_output_devices() -> Result<Vec<AudioDeviceInfo>> {
    Ok(devices(false)?.into_iter().map(|(i, _)| i).collect())
}
pub fn default_microphone() -> Result<AudioDeviceInfo> {
    list_microphones()?
        .into_iter()
        .find(|i| i.is_default)
        .ok_or(AudioError::NoInputDevice)
}
pub fn default_output_device() -> Result<AudioDeviceInfo> {
    list_output_devices()?
        .into_iter()
        .find(|i| i.is_default)
        .ok_or(AudioError::NoOutputDevice)
}
pub(crate) fn select(id: Option<&str>, input: bool) -> Result<cpal::Device> {
    if let Some(id) = id {
        return devices(input)?
            .into_iter()
            .find(|(i, _)| i.id == id)
            .map(|(_, d)| d)
            .ok_or_else(|| AudioError::DeviceUnavailable(id.into()));
    }
    let host = cpal::default_host();
    if input {
        host.default_input_device().ok_or(AudioError::NoInputDevice)
    } else {
        host.default_output_device()
            .ok_or(AudioError::NoOutputDevice)
    }
}
pub fn supported_microphone_configs(id: &str) -> Result<Vec<SupportedAudioConfig>> {
    supported_configs(id, true)
}
pub fn supported_output_configs(id: &str) -> Result<Vec<SupportedAudioConfig>> {
    supported_configs(id, false)
}
fn supported_configs(id: &str, input: bool) -> Result<Vec<SupportedAudioConfig>> {
    let device = select(Some(id), input)?;
    let ranges: Vec<_> = if input {
        device
            .supported_input_configs()
            .map_err(backend_error)?
            .collect()
    } else {
        device
            .supported_output_configs()
            .map_err(backend_error)?
            .collect()
    };
    Ok(ranges
        .into_iter()
        .map(|c| SupportedAudioConfig {
            channels: c.channels(),
            min_sample_rate: c.min_sample_rate().0,
            max_sample_rate: c.max_sample_rate().0,
            sample_format: format!("{:?}", c.sample_format()),
        })
        .collect())
}
