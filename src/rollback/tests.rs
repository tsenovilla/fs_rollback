// SPDX-License-Identifier: GPL-3.0

use super::*;
use std::{
	fs::Permissions,
	io::{Error as IOError, ErrorKind},
	os::unix::fs::PermissionsExt,
};

pub const TEMP_FILE_MESSAGE: &str = "Hello world";
struct TestBuilder {
	temp_file: NamedTempFile,
}

impl TestBuilder {
	fn new() -> Self {
		let file = NamedTempFile::new().expect("Temp file should be created; qed;");
		std::fs::write(&file, TEMP_FILE_MESSAGE).expect("The file exists; qed;");
		Self { temp_file: file }
	}

	fn temp_file_path(&self) -> &Path {
		self.temp_file.path()
	}

	fn execute_with_permissionless_temp_dir<F, T, E>(&self, test: F) -> Result<T, E>
	where
		F: FnOnce() -> Result<T, E>,
	{
		// Save original tempdir locations
		let original_temp_dir = std::env::var("TEMP").unwrap_or_default();
		let original_tmpdir = std::env::var("TMPDIR").unwrap_or_default();

		// Test temp dir
		let test_temp_dir = tempfile::tempdir().expect("Temp dir should be created");

		std::env::set_var("TMPDIR", test_temp_dir.path()); // Unix temp dir
		std::env::set_var("TEMP", test_temp_dir.path()); // Windows temp dir

		// test temp dir is only read
		std::fs::set_permissions(&test_temp_dir, Permissions::from_mode(0o444))
			.expect("temp dir permissions should be configurable; qed;");

		let result = test();

		// Reset tem dir locations
		std::env::set_var("TMPDIR", original_tmpdir);
		std::env::set_var("TEMP", original_temp_dir);

		// Remove test temp dir
		std::fs::set_permissions(&test_temp_dir, Permissions::from_mode(0o755))
			.expect("Permissions should be restored; qed;");
		std::fs::remove_dir_all(test_temp_dir).expect("Test temp dir should be deleted; qed;");

		result
	}
}

#[test]
fn new_creates_an_empty_rollback() {
	let rollback = Rollback::new();

	assert!(rollback.temp_files.is_empty());
	assert!(rollback.noted.is_empty());
	assert!(rollback.new_files.is_empty());
	assert!(rollback.new_dirs.is_empty());
}

#[test]
fn with_capacity_creates_a_rollback_with_capacity() {
	let rollback = Rollback::with_capacity(2, 3, 5);

	assert!(rollback.temp_files.is_empty());
	assert_eq!(rollback.temp_files.capacity(), 2);
	assert!(rollback.noted.is_empty());
	assert!(rollback.noted.capacity() >= 2);
	assert!(rollback.new_files.is_empty());
	assert_eq!(rollback.new_files.capacity(), 3);
	assert!(rollback.new_dirs.is_empty());
	assert_eq!(rollback.new_dirs.capacity(), 5);
}

#[test]
fn note_file_works() {
	let builder = TestBuilder::new();
	let file_path = builder.temp_file_path();

	let mut rollback = Rollback::new();

	let temp_file_path = rollback.note_file("file", file_path);

	assert!(temp_file_path.is_ok());
	let temp_file_path =
		temp_file_path.expect("The previous assertion guaranteees this is OK; qed;");

	// The new file is a copy of the original file
	assert_eq!(
		std::fs::read_to_string(file_path).expect("The file exists; qed;"),
		std::fs::read_to_string(&temp_file_path).expect("The temp file exists; qed;")
	);

	// The new file is recorded under the key "file"
	assert_eq!(
		rollback
			.noted
			.get("file")
			.expect("This entry should exists by the code above; qed;")
			.0,
		temp_file_path
	);
}

#[test]
fn note_file_fails_if_it_cannot_create_temp_file() {
	let builder = TestBuilder::new();
	let file_path = builder.temp_file_path();

	let note_file_result = builder.execute_with_permissionless_temp_dir(|| {
		let mut rollback = Rollback::new();
		rollback.note_file("file", file_path)
	});

	// Note file failed with the expected error
	assert!(note_file_result.is_err());

	match note_file_result
		.err()
		.expect("The previous assertion guarantees the result is err; qed;")
	{
		Error::IO(err) => assert_eq!(err.kind(), ErrorKind::PermissionDenied),
	}
}

#[test]
fn note_file_fails_if_provided_path_doesnt_exist() {
	let mut rollback = Rollback::new();
	let note_file_result = rollback.note_file("file", "some/unexisting/file");

	assert!(note_file_result.is_err());

	match note_file_result
		.err()
		.expect("The previous assertion guarantees the result is err; qed;")
	{
		Error::IO(err) => assert_eq!(err.kind(), ErrorKind::NotFound),
	}
}

#[test]
fn new_file_works() {
	let mut rollback = Rollback::new();

	let temp_dir = tempfile::tempdir().expect("temp dir should be created; qed;");
	let file_path = temp_dir.path().join("file.txt");

	assert!(!file_path.is_file());
	assert!(rollback.new_file(&file_path).is_ok());

	assert!(file_path.is_file());
	assert!(rollback.new_files.contains(&file_path));
}

#[test]
fn new_file_fails_if_file_cannot_be_created() {
	let temp_dir = tempfile::tempdir().expect("temp dir should be created; qed;");
	let file_path = temp_dir.path().join("file.txt");

	// temp dir is only read
	std::fs::set_permissions(&temp_dir, Permissions::from_mode(0o444))
		.expect("temp dir permissions should be configurable; qed;");

	let mut rollback = Rollback::new();
	let new_file_result = rollback.new_file(&file_path);

	assert!(new_file_result.is_err());
	match new_file_result
		.err()
		.expect("The previous assert guarantees the result is Err; qed")
	{
		Error::IO(err) => assert_eq!(err.kind(), ErrorKind::PermissionDenied),
	}
}

#[test]
fn new_dir_works() {
	let mut rollback = Rollback::new();

	let temp_dir = tempfile::tempdir().expect("temp dir should be created; qed;");
	let dir_path = temp_dir.path().join("dir");

	assert!(!dir_path.is_dir());
	assert!(rollback.new_dir(&dir_path).is_ok());

	assert!(dir_path.is_dir());
	assert!(rollback.new_dirs.contains(&dir_path));
}

#[test]
fn new_dir_fails_if_file_cannot_be_created() {
	let temp_dir = tempfile::tempdir().expect("temp dir should be created; qed;");
	let dir_path = temp_dir.path().join("dir");

	// temp dir is only read
	std::fs::set_permissions(&temp_dir, Permissions::from_mode(0o444))
		.expect("temp dir permissions should be configurable; qed;");

	let mut rollback = Rollback::new();
	let new_dir_result = rollback.new_file(&dir_path);

	assert!(new_dir_result.is_err());
	match new_dir_result
		.err()
		.expect("The previous assert guarantees the result is Err; qed")
	{
		Error::IO(err) => assert_eq!(err.kind(), ErrorKind::PermissionDenied),
	}
}

#[test]
fn get_noted_file_works() {
	let builder = TestBuilder::new();
	let file_path = builder.temp_file_path();

	let mut rollback = Rollback::new();

	assert!(rollback.get_noted_file("file").is_none());

	let temp_file_path = rollback.note_file("file", file_path).expect("This should be Ok; qed;");

	assert!(rollback.get_noted_file("file").is_some());
	assert_eq!(
		rollback
			.get_noted_file("file")
			.expect("The previous assert guarantees this is Some; qed"),
		temp_file_path
	);
}

#[test]
fn get_new_file_works() {
	let mut rollback = Rollback::new();

	let temp_dir = tempfile::tempdir().expect("temp dir should be created; qed;");
	let file_path = temp_dir.path().join("file.txt");

	assert!(rollback.get_new_files().is_empty());
	assert!(rollback.new_file(&file_path).is_ok());

	assert_eq!(rollback.get_new_files().len(), 1);
	assert!(rollback.get_new_files().contains(&file_path));
}

#[test]
fn get_new_dir_works() {
	let mut rollback = Rollback::new();

	let temp_dir = tempfile::tempdir().expect("temp dir should be created; qed;");
	let dir_path = temp_dir.path().join("dir");

	assert!(rollback.get_new_dirs().is_empty());
	assert!(rollback.new_dir(&dir_path).is_ok());

	assert_eq!(rollback.get_new_dirs().len(), 1);
	assert!(rollback.get_new_dirs().contains(&dir_path));
}

#[test]
fn commit_works() {
	let builder = TestBuilder::new();
	let file_path = builder.temp_file_path();

	let mut rollback = Rollback::new();

	assert!(rollback.get_noted_file("file").is_none());

	let temp_file_path = rollback.note_file("file", file_path).expect("This should be Ok; qed;");

	// Modify the temp file
	std::fs::write(&temp_file_path, "Modified content").expect("The file should be writable; qed;");

	let file_content = std::fs::read_to_string(file_path).expect("This should be readable; qed");

	assert_eq!(TEMP_FILE_MESSAGE, file_content);

	rollback.commit();

	let file_content = std::fs::read_to_string(file_path).expect("This should be readable; qed");

	assert_eq!("Modified content", file_content);
}

#[test]
fn rollback_works() {
	let mut rollback = Rollback::new();

	let temp_dir = tempfile::tempdir().expect("temp dir should be created; qed;");
	let file_path = temp_dir.path().join("file.txt");
	let dir_path = temp_dir.path().join("dir");

	assert!(rollback.new_file(&file_path).is_ok());
	assert!(rollback.new_dir(&dir_path).is_ok());

	// Check that the file and dir are created
	assert!(file_path.is_file());
	assert!(dir_path.is_dir());

	// rollback
	rollback.rollback();

	// File and dir have been deleted
	assert!(!file_path.is_file());
	assert!(!dir_path.is_dir());
}

#[test]
fn ok_rollback_works() {
	let mut rollback = Rollback::new();

	let temp_dir = tempfile::tempdir().expect("temp dir should be created; qed;");
	let file_path = temp_dir.path().join("file.txt");
	let dir_path = temp_dir.path().join("dir");

	assert!(rollback.new_file(&file_path).is_ok());
	assert!(rollback.new_dir(&dir_path).is_ok());

	// Check that the file and dir are created
	assert!(file_path.is_file());
	assert!(dir_path.is_dir());

	// Ok or rollbakc doesn't rollback if the used result is Ok and returns the inner value
	let ok_result: Result<(), Error> = Ok(());
	let (rollback, result) =
		rollback.ok_or_rollback(ok_result).expect("The used result is Ok; qed;");

	assert_eq!(result, ());

	// The files are still there
	assert!(file_path.is_file());
	assert!(dir_path.is_dir());

	// Rollback if an error is passed in
	let err_result: Result<(), Error> = Err(IOError::new(ErrorKind::NotFound, "ops!").into());
	let error = rollback.ok_or_rollback(err_result);

	match error.err().expect("error is Err; qed;") {
		Error::IO(err) => assert_eq!(err.kind(), ErrorKind::NotFound),
	}

	// File and dir have been deleted
	assert!(!file_path.is_file());
	assert!(!dir_path.is_dir());
}
