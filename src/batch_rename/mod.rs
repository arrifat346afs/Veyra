//! Batch rename: select multiple files and rename them with a template.
//!
//! All logic, state and UI live in this module; the rest of the app only
//! needs to call `open()` and check for `MainWindow::batch_rename`.
mod state;
pub mod view;
#[cfg(test)]
mod tests;

pub use state::BatchRename;
pub use view::{open, view};
