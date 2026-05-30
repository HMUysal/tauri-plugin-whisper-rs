import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

/**
 * Request payload for initializing the Whisper model context.
 */
export interface InitializeRequest {
  /** The absolute path to the Whisper model file (.bin) */
  modelPath: string;
}

/**
 * Request payload for transcribing raw audio data directly from memory.
 */
export interface TranscriptionRequest {
  /** Array of f32 audio samples */
  audioData: number[];
  /** Optional patience factor for beam search */
  patience?: number;
  /** Optional beam size for transcription quality */
  beamSize?: number;
  /** Optional ISO language code (e.g., "tr", "en") */
  language?: string;
}

/**
 * Request payload for transcribing an audio file from the local disk.
 */
export interface TranscriptionFileRequest {
  /** The absolute path to the audio file (MP3, WAV, M4A, etc.) */
  audioPath: string;
  /** Optional patience factor for beam search */
  patience?: number;
  /** Optional beam size for transcription quality */
  beamSize?: number;
  /** Optional ISO language code (e.g., "tr", "en") */
  language?: string;
}

/**
 * Standard response structure for transcription operations.
 */
export interface TranscriptionResponse {
  /** The transcribed text content, if successful */
  text: string | null;
  /** Error message, if the operation failed */
  error: string | null;
}

/**
 * Generic response structure for status-check operations like initialization or release.
 */
export interface GenericResponse {
  /** Indicates whether the operation was successful */
  status: boolean;
  /** Accompanying message or error details */
  message: string;
}

/**
 * Initializes the Whisper context and state with the specified model file.
 * Must be called before running any transcriptions.
 *
 * @param {InitializeRequest} payload - The object containing the model path.
 * @returns {Promise<GenericResponse>} Object indicating success status and message.
 */
export async function initialize(
  payload: InitializeRequest,
): Promise<GenericResponse> {
  return await invoke<GenericResponse>("plugin:whisper-rs|initialize", {
    payload,
  });
}

/**
 * Transcribes raw f32 PCM audio data sent directly from the frontend.
 *
 * @param {TranscriptionRequest} payload - Object containing raw audio array and settings.
 * @returns {Promise<GenericResponse>} The transcription result or error.
 */
export async function transcribe(
  payload: TranscriptionRequest,
): Promise<GenericResponse> {
  return await invoke<GenericResponse>("plugin:whisper-rs|transcribe", {
    payload,
  });
}

/**
 * Instructs the backend to load an audio file from disk, process it, and transcribe it.
 * Highly recommended for larger audio files to maintain optimal performance.
 *
 * @param {TranscriptionFileRequest} payload - Object containing file path and settings.
 * @returns {Promise<GenericResponse>} The transcription result or error.
 */
export async function transcribeFromFile(
  payload: TranscriptionFileRequest,
): Promise<GenericResponse> {
  return await invoke<GenericResponse>(
    "plugin:whisper-rs|transcribe_from_file",
    {
      payload,
    },
  );
}

/**
 * Manually releases the Whisper model and state from memory (RAM/VRAM).
 * Call this when transcription tasks are completely finished to prevent memory leaks.
 *
 * @returns {Promise<GenericResponse>} Object indicating cleanup status.
 */
export async function release(): Promise<GenericResponse> {
  return await invoke<GenericResponse>("plugin:whisper-rs|release");
}

export async function listenTotalProcess(
  callback: (n: number) => void,
): Promise<UnlistenFn> {
  return listen<number>("transcription_total_progress", (r) =>
    callback(r.payload),
  );
}

export async function listenProcess(
  callback: (n: number) => void,
): Promise<UnlistenFn> {
  return listen<number>("transcription_progress", (r) => callback(r.payload));
}

export async function listenTranscription(
  callback: (t: string) => void,
): Promise<UnlistenFn> {
  return listen<string>("transcription", (r) => callback(r.payload));
}
