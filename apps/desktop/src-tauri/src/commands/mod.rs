//! Thin Tauri command surface — no business logic beyond argument plumbing.

mod admin;
mod bureau;
mod dev;
mod live;
mod recovery;
mod settings;
mod training;

pub use admin::*;
pub use bureau::*;
pub use dev::*;
pub use live::*;
pub use recovery::*;
pub use settings::*;
pub use training::*;
