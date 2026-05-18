use crate::models::*;
use serde::de::DeserializeOwned;
use std::fs::File;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader, Track};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use tauri::plugin::PluginApi;
use tauri::{AppHandle, Runtime};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<WhisperRs<R>> {
    Ok(WhisperRs(app.clone()))
}

pub struct WhisperRs<R: Runtime>(AppHandle<R>);

#[derive(Default)]
pub struct AppWhisperState {
    context: Option<WhisperContext>,
    state: Option<WhisperState>,
}

impl AppWhisperState {
    pub fn initialize(&mut self, req: InitializeRequest) -> GenericResponse {
        if !std::path::Path::new(&req.model_path).exists() {
            return GenericResponse {
                status: false,
                message: format!("Model file not found at: {}", req.model_path),
            };
        }

        let ctx_result: Result<WhisperContext, String> =
            WhisperContext::new_with_params(&req.model_path, WhisperContextParameters::default())
                .map_err(|e: whisper_rs::WhisperError| e.to_string());

        self.context = match ctx_result {
            Ok(c) => Some(c),
            Err(e) => {
                return GenericResponse {
                    status: false,
                    message: e,
                }
            }
        };

        if let Some(context) = self.context.as_ref() {
            self.state = match context.create_state().map_err(|e| e.to_string()) {
                Ok(s) => Some(s),
                Err(e) => {
                    return GenericResponse {
                        status: false,
                        message: e,
                    }
                }
            };
            return GenericResponse {
                status: true,
                message: "Success".to_string(),
            };
        }
        return GenericResponse {
            status: false,
            message: "Model could not initialized".to_string(),
        };
    }

    pub fn transcribe(&mut self, req: TranscriptionRequest) -> TranscriptionResponse {
        if let Some(ref mut state) = self.state.as_mut() {
            let strategy: SamplingStrategy = SamplingStrategy::BeamSearch {
                beam_size: 5,
                patience: -1.0,
            };

            let mut params: FullParams = FullParams::new(strategy);
            params.set_language(req.language.as_deref());
            params.set_print_special(false);
            params.set_print_progress(false);

            if let Err(e) = state.full(params, &req.audio_data[..]) {
                return TranscriptionResponse {
                    error: Some(e.to_string()),
                    ..Default::default()
                };
            }

            let mut result_text: String = String::new();
            for segment in state.as_iter() {
                result_text.push_str(&segment.to_string());
            }

            return TranscriptionResponse {
                text: Some(result_text),
                error: None,
                ..Default::default()
            };
        }

        TranscriptionResponse {
            error: Some("Model or State is not initialized".to_string()),
            ..Default::default()
        }
    }

    pub fn transcribe_from_file(&mut self, req: TranscriptionFileRequest) -> TranscriptionResponse {
        if let Some(ref mut state) = self.state.as_mut() {
            let strategy: SamplingStrategy = SamplingStrategy::BeamSearch {
                beam_size: 5,
                patience: -1.0,
            };

            let mut params: FullParams = FullParams::new(strategy);
            params.set_language(req.language.as_deref());
            params.set_print_special(false);
            params.set_print_progress(false);

            let audio_data_result: Result<Vec<f32>, String> = load_audio_data(&req.audio_path);
            let audio_data: Vec<f32> = match audio_data_result {
                Ok(data) => data,
                Err(e) => {
                    return TranscriptionResponse {
                        error: Some(e),
                        ..Default::default()
                    }
                }
            };

            if let Err(e) = state.full(params, &audio_data[..]) {
                return TranscriptionResponse {
                    error: Some(e.to_string()),
                    ..Default::default()
                };
            }

            let mut result_text: String = String::new();
            for segment in state.as_iter() {
                result_text.push_str(&segment.to_string());
            }

            return TranscriptionResponse {
                text: Some(result_text),
                error: None,
                ..Default::default()
            };
        }

        TranscriptionResponse {
            error: Some("Model or State is not initialized".to_string()),
            ..Default::default()
        }
    }

    /**
     * Clears the current Whisper context and state from memory.
     * This effectively drops the model and frees up allocated RAM/VRAM.
     *
     * @returns {GenericResponse} A success message indicating resources are released.
     */
    pub fn release(&mut self) -> GenericResponse {
        self.state = None;
        self.context = None;

        GenericResponse {
            status: true,
            message: "Whisper resources have been successfully released".to_string(),
        }
    }
}

/**
 * Universally loads any audio format and converts it to 16kHz Mono f32 PCM.
 * It uses Symphonia to support various containers (MP3, M4A, WAV, etc.).
 * * @param path The absolute path to the audio file.
 * @return A Result containing a vector of f32 samples or an error string.
 */
fn load_audio_data(path: &str) -> Result<Vec<f32>, String> {
    // 1. Open the file and create a media stream
    let file: File = File::open(path).map_err(|e: std::io::Error| e.to_string())?;
    let mss: MediaSourceStream = MediaSourceStream::new(Box::new(file), Default::default());

    // 2. Provide hints for faster format detection
    let mut hint: Hint = Hint::new();
    if path.ends_with(".mp3") {
        hint.with_extension("mp3");
    } else if path.ends_with(".m4a") {
        hint.with_extension("m4a");
    }

    // 3. Probe the media to find a compatible format reader
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &Default::default())
        .map_err(|e: symphonia::core::errors::Error| e.to_string())?;

    let mut format: Box<dyn FormatReader> = probed.format;

    // 4. Identify the first track with a valid sample rate (indicating an audio track)
    let track: &Track = format
        .tracks()
        .iter()
        .find(|t: &&Track| t.codec_params.sample_rate.is_some())
        .ok_or("No valid audio track found in the provided file")?;

    // 5. Initialize the decoder for the selected track
    let mut decoder: Box<dyn Decoder> = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e: symphonia::core::errors::Error| e.to_string())?;

    let track_id: u32 = track.id;
    let mut pcm_data: Vec<f32> = Vec::new();
    let source_sample_rate: u32 = track.codec_params.sample_rate.unwrap_or(44100);

    // 6. Main decoding loop
    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder.decode(&packet).map_err(|e| e.to_string())?;

        let spec = *decoded.spec();
        let duration = decoded.capacity() as u64;

        let mut sample_buf = SampleBuffer::<f32>::new(duration, spec);

        sample_buf.copy_interleaved_ref(decoded);

        let samples = sample_buf.samples();
        process_raw_samples(
            &mut pcm_data,
            samples,
            spec.channels.count(),
            source_sample_rate,
        );
    }

    Ok(pcm_data)
}

/**
 * Processes interleaved f32 samples: mixes to mono and applies resampling.
 * @param output The main PCM vector to append results.
 * @param samples Raw interleaved samples from SampleBuffer.
 * @param channels Number of audio channels.
 * @param source_rate Original sample rate.
 */
fn process_raw_samples(output: &mut Vec<f32>, samples: &[f32], channels: usize, source_rate: u32) {
    let mut mono_samples = Vec::with_capacity(samples.len() / channels);

    // 1. Mix to Mono: Interleaved veriyi (L, R, L, R...) tek kanala indirger
    for frame in samples.chunks_exact(channels) {
        let sum: f32 = frame.iter().sum();
        mono_samples.push(sum / channels as f32);
    }

    // 2. Resample to 16kHz
    apply_resampling(output, &mono_samples, source_rate);
}

/**
 * Core linear resampling logic to bring any sample rate to 16kHz.
 * Uses linear interpolation for basic audio quality maintenance.
 *
 * @param {&mut Vec<f32>} output - The destination PCM vector.
 * @param {&[f32]} mono_samples - Pre-mixed mono samples in f32 format.
 * @param {u32} source_rate - The original sampling rate of the source audio.
 */
fn apply_resampling(output: &mut Vec<f32>, mono_samples: &[f32], source_rate: u32) {
    let target_rate: f32 = 16000.0;
    let ratio: f32 = source_rate as f32 / target_rate;
    let target_length: usize = (mono_samples.len() as f32 / ratio).floor() as usize;

    output.reserve(target_length);

    for i in 0..target_length {
        let source_index: f32 = i as f32 * ratio;
        let index_low: usize = source_index.floor() as usize;
        let index_high: usize = (index_low + 1).min(mono_samples.len() - 1);

        let weight: f32 = source_index - index_low as f32;

        let sample: f32 =
            (1.0 - weight) * mono_samples[index_low] + weight * mono_samples[index_high];
        output.push(sample);
    }
}
