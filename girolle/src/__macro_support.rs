//! Stable path for symbols that the `#[girolle]` attribute macro
//! emits into user code. The expanded code references
//! `::girolle::__macro_support::*`; nothing else should depend on
//! this module.

pub use crate::service::args::build_inputs_fn_service;
