// SPDX-License-Identifier: GPL-3.0

//! # Definition
//!
//! This crate provides a struct called Rollback, which is useful for controlling changes to the
//! file system. All changes would be discarded unless the rollback is committed.
//!
//! # Example
//!
//! ```ignore
//!     use fs_rollback::Rollback;
//!
//!     // Defines a rollback instance
//!     let mut rollback = Rollback::new();
//!
//!     // Keep registry of this file which is being modified
//!     let roll_file = rollback.note_file(file_path).unwrap();
//!
//!     // Create a new file and keep track of it
//!     let roll_new_file = rollback.new_file(new_file_path).unwrap();
//!     
//!     ....
//!
//!     // If everything went well, we can commit our changes to the fs
//!     rollback.commit();
//! ```

mod error;
mod rollback;

pub use error::Error;
pub use rollback::Rollback;
