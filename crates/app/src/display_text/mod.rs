//! Values as people read them: dates, ages and durations, byte sizes, short
//! commit SHAs and counts with their noun.
//!
//! Files:
//! - `display_date.rs`: `short_date`, a GitHub timestamp as `21 Oct 2026`; ages and durations.
//! - `display_size.rs`: `byte_size`, a byte count as `151.7 MB`.
//! - `short_sha.rs`: `short_sha`, a commit SHA's first seven characters.
//! - `counted.rs`: `counted`, a count with its noun in the singular or plural.

mod counted;
mod display_date;
mod display_size;
mod short_sha;

pub use counted::counted;
pub use display_date::{
    relative_age, short_date, short_duration, time_ago, unix_now, unix_seconds,
};
pub use display_size::byte_size;
pub use short_sha::short_sha;
