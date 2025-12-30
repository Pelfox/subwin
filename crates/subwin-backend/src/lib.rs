//! Backend runtime entry point and public API surface.
//!
//! This crate owns the backend lifecycle, routes bridge messages to services,
//! and manages shared state used by asynchronous tasks.

mod app;
mod config;
mod runtime;
mod services;
mod state;

pub use crate::runtime::run;
