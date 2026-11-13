//! Local Claude Code inventory and one-way messages.
//!
//! - `find_claude.rs`: installed executable lookup, including Finder's sparse PATH.
//! - `git_repository.rs`: main-checkout identity shared by linked worktrees.
//! - `git_command.rs`: local Git command without inherited repository overrides.
//! - `git_branch.rs`: optional read-only branch metadata for a session's checkout.
//! - `command_output.rs`: bounded subprocess output with a deadline.
//! - `session_id.rs`: validated UUID identity, never a display name.
//! - `claude_session.rs`: a snapshot of a running session and its process identity.
//! - `session_inventory.rs`: public CLI discovery joined to the local registry.
//! - `session_sender.rs`: fresh discovery, socket checks and attributed text submission.

mod claude_session;
mod command_output;
mod find_claude;
mod git_branch;
mod git_command;
mod git_repository;
mod session_id;
mod session_inventory;
mod session_sender;

pub use claude_session::ClaudeSession;
pub use find_claude::find_claude;
pub use git_branch::GitBranch;
pub use git_repository::GitRepository;
pub use session_id::SessionId;
pub use session_inventory::SessionInventory;
pub use session_sender::SessionSender;
