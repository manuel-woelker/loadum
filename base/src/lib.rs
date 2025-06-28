// #![allow(clippy::redundant_closure)] // Gives false positives for context! macro

use ecow::EcoString;

pub mod dumper;
pub mod error;
pub mod event;
pub mod result;
pub mod value;

//pub type LoadumString = String;
pub type LoadumString = EcoString;
