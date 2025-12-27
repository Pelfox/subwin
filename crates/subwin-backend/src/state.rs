/// The core application state that holds configuration, caching, and other
/// shared resources.
///
/// This struct contains all the data that needs to be shared across async
/// tasks in the application.
///
/// It is designed to be wrapped in thread-safe, async-friendly concurrency
/// primitives (see [`SharedState`]) to allow safe concurrent reads and
/// occasional writes from multiple tasks.
#[derive(Debug, Clone)]
pub struct State {
    /// The loaded application configuration.
    pub config: subwin_bridge::config::Config,
    /// Path to the directory used for caching data across runs.
    pub cache_path: std::path::PathBuf,
    /// Shared HTTP client for making efficient, pooled requests.
    pub request_client: reqwest::Client,
}

/// Thread-safe, async-friendly shared reference to the application [`State`].
///
/// This is the recommended way to pass state into async handlers, background
/// tasks, or any context where multiple tasks need read access (and occasional
/// write access).
pub type SharedState = std::sync::Arc<tokio::sync::RwLock<State>>;
