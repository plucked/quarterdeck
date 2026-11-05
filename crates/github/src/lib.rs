//! Minimal, blocking GitHub REST client.
//!
//! Blocking on purpose: callers run it on gpui's background executor, which
//! avoids pulling a tokio runtime into the app.

mod auth;
mod client;
mod error;
mod models;

pub use auth::resolve_token;
pub use client::Client;
pub use error::{Error, Result};
pub use models::{PullRequest, PullRequestFile, RepoRef, User};
