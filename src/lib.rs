// SPDX-License-Identifier: GPL-3.0


/// This crates offers a rollback mechanism for rust fs operations 

mod error;
mod rollback;

pub use error::Error;
pub use rollback::Rollback;
