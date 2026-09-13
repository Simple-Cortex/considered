//! Deterministic filesystem traversal, inventory, and source checks.
//!
//! `considered-scan` owns filesystem traversal and must return sorted,
//! workspace-relative paths.

#![allow(clippy::all)]

pub mod inventory;
pub mod lint_source;
pub mod walk;

pub use walk::walk_sorted;
