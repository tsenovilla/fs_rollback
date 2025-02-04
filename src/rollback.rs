// SPDX-License-Identifier: GPL-3.0

mod backup;
mod private_api;
#[cfg(test)]
mod tests;

use crate::Error;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

/// This struct offers a whole rollback mechanism for file system transactions. All operations
/// carried out under the umbrella of a Rollback instance won't affect the fyle system until
/// changes are committed.
///
/// If something goes wrong while committing, every change that has been
/// already made to the file system will be rolled-back.
///
/// All uncommitted changes will be discarded as soon as the instance goes out of scope.
///
/// Rollback uses temporary files under the hood, where every desired change should be applied
/// before committing.
///
/// The struct currently supports:
///  
/// - Modification of existing files.
/// - Creation of new files.
/// - Creation of new directories.
///
/// # Terminology
///
/// - A noted file is an existing file that has been included in the Rollback, with an associated
/// temporary file which is originally a copy of it. Upon commit, the temporary file will override
/// the noted file.
#[derive(Debug)]
pub struct Rollback<'a> {
	// Keep the temp_files inside the rollback struct, so they live until the Rollback dissapears
	temp_files: Vec<NamedTempFile>,
	// Maps original file paths to the temporary file path
	noted: HashMap<&'a Path, PathBuf>,
	// Maps original paths referring files that must be created with its corresponding temporary
	// file
	new_files: HashMap<&'a Path, PathBuf>,
	// New dirs added
	new_dirs: Vec<&'a Path>,
}

impl<'a> Rollback<'a> {
	/// Creates a new, empty instance
	pub fn new() -> Self {
		Self {
			temp_files: Vec::new(),
			noted: HashMap::new(),
			new_files: HashMap::new(),
			new_dirs: Vec::new(),
		}
	}

	/// Creates a new, empty instance with pre allocated memory for noted files, new files and new
	/// directories.
	pub fn with_capacity(
		note_capacity: usize,
		new_files_capacity: usize,
		new_dirs_capacity: usize,
	) -> Self {
		Self {
			temp_files: Vec::with_capacity(note_capacity),
			noted: HashMap::with_capacity(note_capacity),
			new_files: HashMap::with_capacity(new_files_capacity),
			new_dirs: Vec::with_capacity(new_dirs_capacity),
		}
	}

	/// Note an existing file.
	/// ## Errors:
	/// - If the temporary file cannot be created.
	/// - If the temporary file cannot be writen.
	pub fn note_file(&mut self, original: &'a Path) -> Result<(), Error> {
		let temp_file = NamedTempFile::new()?;
		std::fs::copy(original, &temp_file)?;
		self.noted.insert(original, temp_file.path().to_path_buf());
		self.temp_files.push(temp_file);
		Ok(())
	}

	/// Creates a temporary file that will be committed to a new file in the specified path.
	/// The actual new file isn't created until the instance is committed, so trying to access that
	/// file before commit would lead to errors.
	/// ## Errors:
	/// - If the specified path already exists.
	/// - If the temporary file cannot be created.
	/// - If the temporary file cannot be writen.
	pub fn new_file(&mut self, path: &'a Path) -> Result<(), Error> {
		if path.exists() {
			return Err(Error::Descriptive(format!("{:?} already exists", path)));
		}
		let temp_file = NamedTempFile::new()?;
		self.new_files.insert(path, temp_file.path().to_path_buf());
		self.temp_files.push(temp_file);
		Ok(())
	}

	/// Keeps track of a directory that will be created in the specified path upon commit.
	/// ## Errors:
	/// - If the specified path already exists.
	pub fn new_dir(&mut self, path: &'a Path) -> Result<(), Error> {
		if path.exists() {
			return Err(Error::Descriptive(format!("{:?} already exists", path)));
		}
		self.new_dirs.push(path);
		Ok(())
	}

	/// Get the temporary file associated to a noted file.
	pub fn get_noted_file(&self, original: &Path) -> Option<&Path> {
		self.noted.get(original).map(|temp_file| &**temp_file)
	}

	/// Get the temporary file associated to a new file.
	pub fn get_new_file(&self, path: &Path) -> Option<&Path> {
		self.new_files.get(path).map(|temp_file| &**temp_file)
	}

	/// Commit the changes and rollback everything in case of error.
	/// Errors:
	/// - If a noted file cannot be committed. This includes a wide range of possibilities: the
	/// original file doesn't exist anymore, or the proccess doesn't have write permissions on
	/// it,...
	/// - If a new dir cannot be created.
	/// - If a new file cannot be created.
	pub fn commit(self) -> Result<(), Error> {
		let mut backups = Vec::with_capacity(self.noted.capacity());

		match self.commit_noted_files(backups) {
			Ok(computed_backups) => backups = computed_backups,
			Err((err, backups)) => {
				backups.iter().for_each(|backup| backup.rollback());
				return Err(err);
			},
		}

		match self.commit_new_dirs() {
			Err(err) => {
				backups.iter().for_each(|backup| backup.rollback());
				self.rollback_new_dirs();
				return Err(err);
			},
			_ => (),
		}

		match self.commit_new_files() {
			Err(err) => {
				backups.iter().for_each(|backup| backup.rollback());
				self.rollback_new_files();
				self.rollback_new_dirs();
				return Err(err);
			},
			_ => (),
		}

		Ok(())
	}
}
