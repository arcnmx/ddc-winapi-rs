//! TODO: use the `registry` crate instead [once it moves from winapi][registry-issue]
//!
//! [registry-issue]: https://github.com/bbqsrc/registry-rs/issues/9

pub use self::key::Key;

mod key;
