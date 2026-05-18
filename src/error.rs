use serde::{ser::Serializer, Serialize};

/**
 * A specialized Result type for the Whisper plugin operations.
 */
pub type Result<T> = std::result::Result<T, Error>;

/**
 * Core Error enum representing all possible failures within the plugin.
 * Uses `thiserror` for automatic display formatting and trait derivations.
 */
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[cfg(mobile)]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),

    // Added: To handle ad-hoc or generic string errors across the plugin
    #[error("Whisper plugin error: {0}")]
    Generic(String),
}

/**
 * Enables implicit conversion from String to our custom Error using the `?` operator.
 */
impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Generic(s)
    }
}

/**
 * Custom serialization to ensure errors are sent back to the frontend as clean strings.
 */
impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let error_string: String = self.to_string();
        serializer.serialize_str(error_string.as_ref())
    }
}
