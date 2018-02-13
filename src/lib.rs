//! A crate to count ones and xor bytes, fast (aka popcount, hamming
//! weight and hamming distance).
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
//! assert_eq!(hamming::distance(&[1, 0xFF], &[0xFF, 1]), 7 + 7);
//! ```

#![cfg_attr(all(test, feature = "unstable"), feature(test))]
#[cfg(test)] extern crate quickcheck;
#[cfg(test)] extern crate rand;
#[cfg(all(test, feature = "unstable"))] extern crate test;
#[cfg(feature = "simd")] extern crate faster;

mod weight_;
pub use weight_::weight;

mod distance_;
pub use distance_::{distance, distance_fast};
