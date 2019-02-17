//! A tower hyper bridge library that provides utilties
//! to use hyper with a tower middleware stack.
//!
//! # Overview
//!
//! This library is comprised of client and server modules. Currently, only
//! the client portion is done and working. The server side is blocked partially
//! by hypers use of its own Service and MakeService traits.

#![deny(missing_docs, missing_debug_implementations)]

pub mod client;
/// Contains retry utilities
pub mod retries;
/// Contains general utilities
pub mod util;
/// Contains the specialized body for retries
pub mod body;

pub use body::Body;
