// SPDX-License-Identifier: GPL-3.0

use super::*;
use crate::test_builder::{
	TestBuilder, MODIFIED_BUILDER_FILE_CONTENT, ORIGINAL_BUILDER_FILE_CONTENT,
};
use std::io::ErrorKind;

#[test]
fn new_backup_works() {
	let builder = TestBuilder::new(Some(1));
	let file_path = builder.existing_files().remove(0);
	let backup = Backup::new(file_path).expect("The backup should be created; qed;");

	assert_eq!(backup.original, file_path);
	assert_eq!(
		std::fs::read_to_string(backup.backup).expect("The backup should be readable; qed;"),
		ORIGINAL_BUILDER_FILE_CONTENT
	);
}

#[test]
fn new_backup_fails_if_temporary_file_cannot_be_created() {
	TestBuilder::new(Some(1)).with_read_only_dir().execute(|builder, _| {
		match Backup::new(builder.existing_files()[0]) {
			Err(Error::IO(err)) if err.kind() == ErrorKind::PermissionDenied => (),
			_ => assert!(false),
		}
	});
}

#[test]
fn new_backup_fails_if_original_doesnt_exist() {
	let result = Backup::new(&PathBuf::from("some/unexisting/path/file.txt"));

	match result {
		Err(Error::IO(err)) if err.kind() == ErrorKind::NotFound => (),
		_ => assert!(false),
	}
}

#[test]
fn backup_rollback_works() {
	let builder = TestBuilder::new(Some(1));
	let file_path = builder.existing_files()[0];
	let backup = Backup::new(file_path).expect("The backup should be created; qed;");

	std::fs::write(file_path, MODIFIED_BUILDER_FILE_CONTENT)
		.expect("The file path should be writable; qed;");

	assert_eq!(
		std::fs::read_to_string(file_path).expect("File should be readable; qed;"),
		MODIFIED_BUILDER_FILE_CONTENT
	);

	backup.rollback();

	assert_eq!(
		std::fs::read_to_string(file_path).expect("File should be readable; qed;"),
		ORIGINAL_BUILDER_FILE_CONTENT
	);
}
