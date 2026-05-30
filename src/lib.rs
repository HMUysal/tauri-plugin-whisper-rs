use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

mod audio_processor;
mod commands;
mod error;
mod models;
mod whisper;

use crate::whisper::{AppWhisperState, WhisperRs};
pub use error::{Error, Result};
use std::sync::Mutex;

pub trait WhisperRsExt<R: Runtime> {
    fn whisper_rs(&self) -> &WhisperRs<R>;
}

impl<R: Runtime, T: Manager<R>> crate::WhisperRsExt<R> for T {
    fn whisper_rs(&self) -> &WhisperRs<R> {
        self.state::<WhisperRs<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Debug)
            .with_tag("TAURI_RUST"),
    );
    Builder::new("whisper-rs")
        .invoke_handler(tauri::generate_handler![
            commands::initialize,
            commands::transcribe,
            commands::transcribe_from_file,
            commands::release,
        ])
        .setup(|app, api| {
            app.manage(Mutex::new(AppWhisperState::default()));

            let whisper_rs = whisper::init(app, api)?;
            app.manage(whisper_rs);
            Ok(())
        })
        .build()
}
