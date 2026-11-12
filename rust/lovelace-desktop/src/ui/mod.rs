//! The native egui workbench window (eframe/egui 0.37) and its panes.
//!
//! The measured prototype in `rust/lovelace-desktop-spike` is the source of the editor,
//! syntax, plot and layout code; this module adapts those pieces to the product's
//! [`crate::workbench::Workbench`] instead of the spike's sample documents, sample variables
//! and sample SVG.

mod app;
pub(crate) mod editor;
pub(crate) mod plot;
mod syntax;

pub use app::run;
