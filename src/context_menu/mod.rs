//! Background directory menu. Layout, commands and filesystem work live separately.
pub mod actions;
mod clipboard;
pub mod dialog;
mod files;
mod view;
pub use view::{attach, attach_entry};
