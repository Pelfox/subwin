//! Backend service handlers for frontend-driven requests.
//!
//! This module groups async request handlers that operate on the shared
//! `AppContext`, perform side effects (network, filesystem), and emit progress
//! or notifications back to the frontend.

pub mod audio_service;
pub mod config_service;
pub mod model_service;
pub mod transcription_service;

/// Represents a type that is used in all handlers as an application context.
pub(crate) type AppContextHandle = std::sync::Arc<crate::AppContext>;
