//! A crate to count ones, fast (aka popcount, hamming weight).
//!
//! # Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! hamming = "0.1"
//! ```
//!
//! # Examples
//!
//! ```rust
//! assert_eq!(hamming::weight(&[1, 0xFF, 1, 0xFF]), 1 + 8 + 1 + 8);
//! ```

#![cfg_attr(all(test, feature = "unstable"), feature(test))]
#[cfg(test)] extern crate quickcheck;
#[cfg(test)] extern crate rand;
#[cfg(all(test, feature = "unstable"))] extern crate test;

mod weight_;
pub use weight_::weight;
