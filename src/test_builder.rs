// SPDX-License-Identifier: GPL-3.0

use crate::Rollback;
use std::{
	fs::Permissions,
	os::unix::fs::PermissionsExt,
	path::{Path, PathBuf},
};
use tempfile::{NamedTempFile, TempDir};

pub const ORIGINAL_BUILDER_FILE_CONTENT: &str = "Hello world";
pub const MODIFIED_BUILDER_FILE_CONTENT: &str = "This is me";
// The default number of files created by the builder.
const BUILDER_CAPACITY: usize = 10;

// A useful struct for testing this crate
pub struct TestBuilder {
	// The builder's capacity, this is, the number of existing files, new files paths and new dirs
	// paths created with the builder
	capacity: usize,
	// A tempdir where everything happens inside the builder. Everything's cleaned out as soon as
	// the builder instance goes out of scope
	tempdir: TempDir,
	// #{capacity} files created inside tempdir.
	existing_files: Vec<NamedTempFile>,
	// #{capacity} valid file paths inside the tempdir.
	new_files: Vec<PathBuf>,
	// #{capacity} valid directory paths inside the tempdir.
	new_dirs: Vec<PathBuf>,
	// Specify if the rollback passed to the execute method should note the existing files.
	with_noted_files: bool,
	// Specify if the rollback passed to the execute method should note the new files.
	with_new_files: bool,
	// Specify if the rollback passed to the execute method should note the new dirs.
	with_new_dirs: bool,
	// Specify if the tempdir should be read only
	with_read_only_dir: bool,
	// Specify if the system temp dir should be read only
	with_read_only_temp_dir: bool,
	// Specify if the existing files shouldn't have any permissions
	with_permissionless_files: bool,
}

impl TestBuilder {
	// Creates a new instance of the builder using the specified capacity or the default one.
	pub fn new(capacity: Option<usize>) -> Self {
		let capacity = capacity.unwrap_or(BUILDER_CAPACITY);
		let mut existing_files = Vec::with_capacity(capacity);
		let mut new_files = Vec::with_capacity(capacity);
		let mut new_dirs = Vec::with_capacity(capacity);
		let tempdir = tempfile::tempdir().expect("Tempdir should be created; qed");
		for i in 0..capacity {
			let file =
				NamedTempFile::new_in(tempdir.path()).expect("Temp file should be created; qed;");
			std::fs::write(&file, ORIGINAL_BUILDER_FILE_CONTENT).expect("The file exists; qed;");
			existing_files.push(file);
			new_files.push(tempdir.path().join(format!("{}.txt", i)));

			new_dirs.push(tempdir.path().join(i.to_string()));
		}
		Self {
			capacity,
			tempdir,
			existing_files,
			new_files,
			new_dirs,
			with_noted_files: false,
			with_new_files: false,
			with_new_dirs: false,
			with_read_only_dir: false,
			with_read_only_temp_dir: false,
			with_permissionless_files: false,
		}
	}

	pub fn existing_files(&self) -> Vec<&Path> {
		self.existing_files.iter().map(|file| file.path()).collect()
	}

	pub fn new_files(&self) -> Vec<&Path> {
		self.new_files.iter().map(|path| path.as_path()).collect()
	}

	pub fn new_dirs(&self) -> Vec<&Path> {
		self.new_dirs.iter().map(|path| path.as_path()).collect()
	}

	pub fn with_noted_files(mut self) -> Self {
		self.with_noted_files = true;
		self
	}

	pub fn with_new_files(mut self) -> Self {
		self.with_new_files = true;
		self
	}

	pub fn with_new_dirs(mut self) -> Self {
		self.with_new_dirs = true;
		self
	}

	pub fn with_read_only_dir(mut self) -> Self {
		self.with_read_only_dir = true;
		self
	}

	pub fn with_read_only_temp_dir(mut self) -> Self {
		self.with_read_only_temp_dir = true;
		self
	}

	pub fn with_permissionless_files(mut self) -> Self {
		self.with_permissionless_files = true;
		self
	}

	pub fn capacity(&self) -> usize {
		self.capacity
	}

	// Executes the inner closure with pre configured TestBuilder and Rollback instances according
	// to the TestBuilder instance.
	pub fn execute<'a, F>(&'a self, test: F)
	where
		F: Fn(&'a Self, Rollback<'a>) -> (),
	{
		let mut rollback = Rollback::with_capacity(self.capacity, self.capacity, self.capacity);

		if self.with_noted_files {
			self.existing_files.iter().for_each(|file| {
				rollback.note_file(file.path()).expect("The file should be noted; qed;");
			});

			self.existing_files
				.iter()
				.map(|file| rollback.get_noted_file(file.path()).expect("The files is noted; qed;"))
				.for_each(|file| {
					std::fs::write(file, MODIFIED_BUILDER_FILE_CONTENT)
						.expect("The file exists and are writable; qed;")
				});

			// Modifying the rollback doesn't modify the original
			self.existing_files.iter().for_each(|file| {
				assert_eq!(
					std::fs::read_to_string(file).expect("Files exist and are readable; qed"),
					ORIGINAL_BUILDER_FILE_CONTENT
				)
			});
		}

		if self.with_new_files {
			self.new_files.iter().for_each(|file| {
				rollback
					.new_file(&file)
					.expect("New files should be correctly added to the rollback; qed;")
			});

			self.new_files
				.iter()
				.map(|file| {
					rollback
						.get_new_file(&file)
						.expect("The new file belongs to the rollback; qed;")
				})
				.for_each(|file| {
					std::fs::write(file, ORIGINAL_BUILDER_FILE_CONTENT)
						.expect("The file exists and is writable; qed;")
				});
		}

		if self.with_new_dirs {
			self.new_dirs.iter().for_each(|dir| {
				rollback
					.new_dir(&dir)
					.expect("New dire should be correctly added to the rollback; qed;")
			});
		}

		if self.with_read_only_dir {
			std::fs::set_permissions(self.tempdir.path(), Permissions::from_mode(0o555))
				.expect("temp dir permissions should be configurable; qed;");
		}

		if self.with_read_only_temp_dir {
			std::env::set_var("TMPDIR", self.tempdir.path());
			std::env::set_var("TEMP", self.tempdir.path());

			std::fs::set_permissions(self.tempdir.path(), Permissions::from_mode(0o555))
				.expect("temp dir permissions should be configurable; qed;");
		}

		if self.with_permissionless_files {
			self.existing_files.iter().for_each(|file| {
				std::fs::set_permissions(file, Permissions::from_mode(0o000))
					.expect("File permissions should be configurable; qed;");
			});
		}

		test(self, rollback);
	}
}
