//! GitHub avatars: downloaded and decoded in the background, kept for the
//! session, drawn round with a placeholder until they arrive.
//!
//! Files:
//! - `avatar_images.rs`: `AvatarImages`, the global store of avatars by URL, and their loading.
//! - `avatar.rs`: `Avatar`, a user's round avatar.

mod avatar;
mod avatar_images;

pub use avatar::Avatar;
pub use avatar_images::AvatarImages;
