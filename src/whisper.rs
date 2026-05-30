use crate::audio_processor::AudioProcessor;
use crate::models::*;
use log::error;
use serde::de::DeserializeOwned;
use tauri::plugin::PluginApi;
use tauri::{AppHandle, Emitter, Runtime};
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

    pub fn transcribe<R: Runtime>(&mut self, app: AppHandle<R>, req: TranscriptionRequest) {
        let Some(state) = &mut self.state else {
            return;
        };

        let strategy = SamplingStrategy::BeamSearch {
            beam_size: req.beam_size.unwrap_or(1),
            patience: req.patience.unwrap_or(-1.00),
        };
        let mut params = FullParams::new(strategy);
        params.set_language(req.language.as_deref());
        params.set_print_special(false);
        params.set_print_progress(false);

        if let Err(e) = state.full(params, &req.audio_data) {
            error!("Whisper inference error: {:?}", e);
            let _ = app.emit("transcription_error", e.to_string());
            return;
        }

        let seconds_processed = req.audio_data.len() as f32 / 16000.0;
        let _ = app.emit("transcription_progress", seconds_processed);

        let result_text: String = state.as_iter().map(|s| s.to_string()).collect();
        let _ = app.emit("transcription", result_text.clone());
    }

    pub fn transcribe_from_file<R: Runtime>(
        &mut self,
        app: AppHandle<R>,
        req: TranscriptionFileRequest,
    ) {
        let patience = req.patience;
        let beam_size = req.beam_size;
        let chunk_suze = req.chunk_size.unwrap_or(30);
        let language = req.language.clone();

        let mut ap = AudioProcessor::new(app.clone());
        let _ = ap.set_file(&req.audio_path);
        let _ = ap.set_file_info();
        let _ = ap.set_chunk_target_seconds(chunk_suze);

        let callback = move |data: Vec<f32>| {
            let r = TranscriptionRequest {
                audio_data: data,
                patience,
                beam_size,
                language: language.clone(),
            };

            self.transcribe(app.clone(), r);
        };
        let _ = ap.start_decoding(callback);
    }

    pub fn release(&mut self) -> GenericResponse {
        self.state = None;
        self.context = None;

        GenericResponse {
            status: true,
            message: "Whisper resources have been successfully released".to_string(),
        }
    }
}
