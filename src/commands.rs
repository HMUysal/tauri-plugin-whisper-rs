use crate::{models::*, whisper::AppWhisperState, Result};
use std::sync::Mutex;
use tauri::{command, State};

/**
 * Initializes the Whisper context and state with the provided model path.
 *
 * @param {State<'_, Mutex<AppWhisperState>>} state - The managed plugin state.
 * @param {InitializeRequest} payload - Contains the path to the model file.
 * @returns {Result<GenericResponse>} Returns status true on success, or false with error message.
 */
#[command]
pub fn initialize(
    state: State<'_, Mutex<AppWhisperState>>,
    payload: InitializeRequest,
) -> Result<GenericResponse> {
    // Lock the mutex to get mutable access to the state
    let mut state: std::sync::MutexGuard<'_, AppWhisperState> =
        state.lock().map_err(|e| e.to_string())?;
    let result: GenericResponse = state.initialize(payload);
    Ok(result)
}

/**
 * Transcribes raw audio data (f32 PCM) using the initialized Whisper state.
 *
 * @param {State<'_, Mutex<AppWhisperState>>} state - The managed plugin state.
 * @param {TranscriptionRequest} payload - Contains the raw audio samples and optional settings.
 * @returns {Result<TranscriptionResponse>} The transcribed text or error details.
 */
#[command]
pub fn transcribe(
    state: State<'_, Mutex<AppWhisperState>>,
    payload: TranscriptionRequest,
) -> Result<TranscriptionResponse> {
    let mut state: std::sync::MutexGuard<'_, AppWhisperState> =
        state.lock().map_err(|e| e.to_string())?;
    let result: TranscriptionResponse = state.transcribe(payload);
    Ok(result)
}

/**
 * Loads an audio file from the disk, processes it, and returns the transcription.
 *
 * @param {State<'_, Mutex<AppWhisperState>>} state - The managed plugin state.
 * @param {TranscriptionFileRequest} payload - Contains the file path and optional settings.
 * @returns {Result<TranscriptionResponse>} The transcribed text from the file or error details.
 */
#[command]
pub fn transcribe_from_file(
    state: State<'_, Mutex<AppWhisperState>>,
    payload: TranscriptionFileRequest,
) -> Result<TranscriptionResponse> {
    let mut state: std::sync::MutexGuard<'_, AppWhisperState> =
        state.lock().map_err(|e| e.to_string())?;
    let result: TranscriptionResponse = state.transcribe_from_file(payload);
    Ok(result)
}

/**
 * Command to manually release the model from memory when no longer needed.
 *
 * @param {State<'_, Mutex<AppWhisperState>>} state - The managed plugin state.
 * @returns {Result<GenericResponse>} Success status after memory cleanup.
 */
#[command]
pub fn release(state: State<'_, Mutex<AppWhisperState>>) -> Result<GenericResponse> {
    let mut state: std::sync::MutexGuard<'_, AppWhisperState> =
        state.lock().map_err(|e| e.to_string())?;
    let result: GenericResponse = state.release();
    Ok(result)
}
