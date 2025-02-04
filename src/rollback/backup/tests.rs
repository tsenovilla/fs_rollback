// SPDX-License-Identifier: GPL-3.0

use super::*;
use crate::rollback::tests::{TestBuilder, BUILDER_FILE_CONTENT};
use std::io::ErrorKind;

#[test]
fn new_backup_works() {
	let builder = TestBuilder::new();
	let backup = Backup::new(builder.file_path()).expect("The backup should be created; qed;");

	assert_eq!(backup.original, builder.file_path());
	assert_eq!(
		std::fs::read_to_string(backup.backup).expect("The backup should be readable; qed;"),
      BUILDER_FILE_CONTENT
	);
}

#[test]
fn new_backup_fails_if_temporary_file_cannot_be_created(){
    let builder = TestBuilder::new();
   let file_path = builder.file_path();

    let result = builder.execute_with_permissionless_temp_dir(|| Backup::new(file_path));

    match result{
        Err(Error::IO(err)) if err.kind() == ErrorKind::PermissionDenied => (),
        _ => assert!(false)
    }
}

#[test]
fn new_backup_fails_if_original_doesnt_exist(){
    let result = Backup::new(&PathBuf::from("some/unexisting/path/file.txt"));

    match result{
        Err(Error::IO(err)) if err.kind() == ErrorKind::NotFound => (),
        _ => assert!(false)
    }
}

#[test]
fn backup_rollback_works(){
    let builder = TestBuilder::new();
    let file_path = builder.file_path();
    let backup = Backup::new(file_path).expect("The backup should bee created; qed;");

    std::fs::write(file_path, "Modified").expect("The file path should be writable; qed;");

    assert_eq!(std::fs::read_to_string(file_path).expect("File should be readable; qed;"), "Modified");

    backup.rollback();

    assert_eq!(std::fs::read_to_string(file_path).expect("File should be readable; qed;"), BUILDER_FILE_CONTENT);

}
