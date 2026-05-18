use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequest {
    pub model_path: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeStateRequest {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionRequest {
    pub audio_data: Vec<f32>,
    pub patience: Option<f32>,
    pub beam_size: Option<i32>,
    pub language: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionFileRequest {
    pub audio_path: String,
    pub patience: Option<f32>,
    pub beam_size: Option<i32>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResponse {
    pub text: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericResponse {
    pub status: bool,
    pub message: String,
}
