// SPDX-License-Identifier: GPL-3.0

use thiserror::Error;

/// Represents the various errors that can occur in the crate.
///
/// Many errors occur cause an invalid [`Path`] is provided to the rollback. The related errors
/// contain those [`Path`] as [`String`] to provide user-friendly error messages.
#[derive(Error, Debug)]
pub enum Error {
	/// An path is already noted by the rollback.
	#[error("{0} has been already noted by this rollback.")]
	AlreadyNoted(String),
	/// An item couldn't be committed. Contains the path to the affected item and the error as
	/// [`String`].
	#[error("Commiting {0} failed with error: {1}.")]
	Commit(String, String),
	#[error("IO error: {0}")]
	IO(#[from] std::io::Error),
	/// A path marked as 'new' for this rollback already exists.
	#[error("{0} already exists and cannot be noted as 'new'.")]
	NewItemAlreadyExists(String),
	/// A path doesn't represent a directory.
	#[error("{0} isn't a dir.")]
	NotADir(String),
	/// A path doesn't represent a file.
	#[error("{0} isn't a file.")]
	NotAFile(String),
	/// A path has been declared as new dir several times.
	#[error("The path {0} has been noted several times as new_dir.")]
	RepeatedNewDir(String),
	/// A path has been declared as new file several times.
	#[error("The path {0} has been noted several times as new_file.")]
	RepeatedNewFile(String),
}
