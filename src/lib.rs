//! Library crate for the Actix Web service.
//!
//! `main.rs` stays thin; everything else lives here so it can be reused by
//! integration tests and other binaries.

pub mod config;
pub mod db;
pub mod error;
pub mod modules;
pub mod startup;
pub mod utils;
