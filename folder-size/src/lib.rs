//! Folder size analysis for Cardinal.
//!
//! * [`scan`] measures allocated size (like `du`) with `getattrlistbulk`, in parallel.
//! * [`tree`] keeps per-directory totals and supports in-place subtree updates.
//! * [`engine::SizeEngine`] owns the tree, runs scans and applies FSEvents
//!   incrementally so sizes stay current without full rescans.
//! * [`suggest`] lists well-known cache/temporary locations; it never deletes.

pub mod apps;
pub mod engine;
pub mod orphans;
pub mod persist;
pub mod scan;
pub mod suggest;
pub mod tree;
pub mod volume;

pub use engine::{Phase, SizeEngine, Status};
pub use suggest::{Level, Suggestion};
pub use tree::{Dir, FileEntry, ItemKind, ListItem, Listing, SizeTree};
pub use volume::{VolumeSpace, volume_space};
