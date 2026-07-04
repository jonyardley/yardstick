//! MCP surface for Daily: an rmcp streamable-HTTP server exposing
//! `ping`, `create_task`, and `list_tasks` behind bearer-token auth.
//!
//! This crate defines the seams the runtime implements in Task 7:
//! [`EventSink`] (dispatch events into the Crux core) and [`TaskReader`]
//! (read the current task list).

mod auth;
mod server;

pub use server::{DailyMcp, serve_http, serve_http_on};

use shared::{Event, Task};

/// Dispatches events into the Crux core. Implemented by `runtime`.
pub trait EventSink: Send + Sync {
    fn send_event(&self, event: Event);
}

/// Reads the current task list. Implemented by `runtime` over `store`.
pub trait TaskReader: Send + Sync {
    fn list_tasks(&self) -> Result<Vec<Task>, String>;
}
