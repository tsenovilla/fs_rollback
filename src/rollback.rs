// SPDX-License-Identifier: GPL-3.0

mod backup;
mod private_api;
#[cfg(all(test, not(feature = "integration-tests")))]
mod tests;

use crate::Error;
use std::{collections::HashMap, path::Path};
use tempfile::NamedTempFile;

/// # Description
///
/// This struct offers a whole rollback mechanism for file system transactions. All operations
/// carried out under the umbrella of a Rollback instance won't affect the file system until
/// changes are committed.
///
/// If something goes wrong while committing, every change that has been managed by the instance
/// will be rolled-back, while all uncommitted changes will be discarded as soon as the instance
/// goes out of scope.
///
/// Rollback uses temporary files under the hood. Be sure to apply your changes to those files and
/// then commit the Rollback instance to unleash the full power of Rollback.
///
/// The struct currently supports:
///  
/// - Modification of existing files.
/// - Creation of new files.
/// - Creation of new directories.
///
/// # Considerations
///
/// - When a file is added to the rollback 'to be modified', or when a new file is added as 'to be
///   created', a temporary file is created an remains open until the Rollback instance goes out of
///   scope (a commit consumes the rollback). While it's unlikely that the limit of open files is
///   reached, this is something worth to keep in mind.
///
///   Those temporary files are created using the tempfile crate, so the security considerations
///   described [here](https://docs.rs/tempfile/latest/tempfile/) applies for this crate as well.
///
/// - The [`Rollback`] struct is able to detect if two different paths point to the same file when
///   its added to the rollback to be modified. However, there's not a way to detect if two paths
///   will resolve to the same new file or new directory, so it will accept both.
///
///   In the first case, the rollback simply won't accept the noted file the second time. In the
///   second case, however, the rollback will accept the path and **fail** when it's committed, to
///   avoid a race condition. Note that this mean that all modifications will be rolled-back, so
///   paying attention to new files/new dirs paths is crutial.

#[derive(Debug)]
pub struct Rollback<'a> {
	// Maps original file paths to the temporary file. As the temporary file is included in the
	// map, it lives as long as the instance does.
	noted: HashMap<&'a Path, NamedTempFile>,
	// Maps original paths referring files that must be created with its corresponding temporary
	// file. As the temporary file is included in the map, it lives as long as the instance does.
	new_files: HashMap<&'a Path, NamedTempFile>,
	// New dirs added.
	new_dirs: Vec<&'a Path>,
}

impl Default for Rollback<'_> {
	/// Creates a new, empty instance
	fn default() -> Self {
		Self { noted: HashMap::new(), new_files: HashMap::new(), new_dirs: Vec::new() }
	}
}

impl<'a> Rollback<'a> {
	/// Creates a new, empty instance with pre allocated memory for noted files, new files and new
	/// directories.
	pub fn with_capacity(
		note_capacity: usize,
		new_files_capacity: usize,
		new_dirs_capacity: usize,
	) -> Self {
		Self {
			noted: HashMap::with_capacity(note_capacity),
			new_files: HashMap::with_capacity(new_files_capacity),
			new_dirs: Vec::with_capacity(new_dirs_capacity),
		}
	}

	/// Registers an existing file as 'to be modified', creating a temporary file that will be
	/// committed to the existing file upon commit.
	/// ## Errors:
	/// - If the file is already noted, either using exactly the same [`Path`] or a different
	///   representation of it.
	/// - If the original path isn't a file.
	/// - If the temporary file cannot be created.
	/// - If the temporary file cannot be writen.
	pub fn note_file(&mut self, original: &'a Path) -> Result<(), Error> {
		if !original.is_file() {
			return Err(Error::NotAFile(format!("{}", original.display())));
		} else if self
			.noted
			.keys()
			.any(|path| same_file::is_same_file(original, path).unwrap_or(false))
		{
			return Err(Error::AlreadyNoted(format!("{}", original.display())));
		}

		// Committing the noted files cannot just persist the temp files as they live inside the
		// Rollback instance, so moving them out isn't possible, but copying its content is.
		// Hence, the tempfile can be created in the default temp dir.
		let temp_file = NamedTempFile::new()?;
		std::fs::copy(original, &temp_file)?;
		self.noted.insert(original, temp_file);
		Ok(())
	}

	/// Registers a valid file path as 'to be created', creating a temporary file that will be
	/// committed to this new file. The actual new file isn't created until the Rollback instance
	/// is committed, so trying to access it would lead to errors.
	/// ## Considerations:
	/// - If creating a file whose parent dir doesn't exist, consider adding that path to the
	///   instance as well using the `new_dir` method. Otherwise, the rollback wouldn't be able to
	///   commit the new file.
	///
	/// ## Errors:
	/// - If the specified path already exists.
	/// - If the path is already noted.
	/// - If the path isn't a valid file path.
	/// - If the temporary file cannot be created.
	pub fn new_file(&mut self, path: &'a Path) -> Result<(), Error> {
		if path.exists() {
			return Err(Error::NewItemAlreadyExists(format!("{}", path.display())));
		} else if self.new_files.contains_key(path) {
			return Err(Error::AlreadyNoted(format!("{}", path.display())));
		} else if path.extension().is_none() {
			return Err(Error::NotAFile(format!("{}", path.display())));
		}

		// Committing the new files cannot just persist the temp files as they live inside the
		// Rollback instance, so moving them out isn't possible, but copying its content is.
		// Hence, the tempfile can be created in the default temp dir.
		self.new_files.insert(path, NamedTempFile::new()?);
		Ok(())
	}

	/// Registers a valid direcroty path as 'to be created'. The directory isn't created until the
	/// Rollback instance is committed, so trying to access it would lead to errors.
	/// ## Errors:
	/// - If the specified path already exists.
	/// - If the specified path is already noted.
	/// - If the path isn't a valid directory path.
	pub fn new_dir(&mut self, path: &'a Path) -> Result<(), Error> {
		if path.exists() {
			return Err(Error::NewItemAlreadyExists(format!("{}", path.display())));
		} else if self.new_dirs.contains(&path) {
			return Err(Error::AlreadyNoted(format!("{}", path.display())));
		} else if path.as_os_str().is_empty() || path.extension().is_some() {
			return Err(Error::NotADir(format!("{}", path.display())))
		}
		self.new_dirs.push(path);
		Ok(())
	}

	/// Get the temporary file associated to a noted file.
	pub fn get_noted_file<P: AsRef<Path>>(&self, original: P) -> Option<&Path> {
		self.noted.get(original.as_ref()).map_or_else(
			|| {
				self.noted.keys().find_map(|path| {
					same_file::is_same_file(path, original.as_ref())
						.ok()
						.filter(|&same| same)
						.and_then(|_| self.noted.get(path).map(|temp_file| temp_file.path()))
				})
			},
			|temp_file| Some(temp_file.path()),
		)
	}

	/// Get the temporary file associated to a new file.
	pub fn get_new_file<P: AsRef<Path>>(&self, path: P) -> Option<&Path> {
		self.new_files.get(path.as_ref()).map(|temp_file| temp_file.path())
	}

	/// Consume the Rollback and commit the changes. If something goes wrong during the commit step,
	/// everything is rolled-back, so the file system isn't affected.
	///
	/// ## Errors:
	/// - If a noted file cannot be committed. This includes a wide range of possibilities: the
	///   original file doesn't exist anymore, or the proccess doesn't have write permissions on
	///   it,...
	/// - If a new dir cannot be created.
	/// - If a new file cannot be created.
	pub fn commit(self) -> Result<(), Error> {
		let mut backups = Vec::with_capacity(self.noted.capacity());

		match self.commit_noted_files(backups) {
			Ok(computed_backups) => backups = computed_backups,
			Err((err, backups)) => {
				backups.into_iter().for_each(|backup| backup.rollback());
				return Err(err);
			},
		}

		if let Err(err) = self.commit_new_dirs() {
			backups.into_iter().for_each(|backup| backup.rollback());
			self.rollback_new_dirs();
			return Err(err);
		}

		if let Err(err) = self.commit_new_files() {
			backups.into_iter().for_each(|backup| backup.rollback());
			self.rollback_new_files();
			self.rollback_new_dirs();
			return Err(err);
		}

		Ok(())
	}
}
