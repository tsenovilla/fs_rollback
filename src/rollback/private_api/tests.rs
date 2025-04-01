// SPDX-License-Identifier: GPL-3.0

use super::*;
use crate::test_builder::{
	TestBuilder, MODIFIED_BUILDER_FILE_CONTENT, ORIGINAL_BUILDER_FILE_CONTENT,
};
use std::path::Path;

#[test]
fn rollback_new_dirs_works() {
	TestBuilder::new(None).with_new_dirs().execute(|builder, rollback| {
		// Create one of the new dirs
		std::fs::create_dir_all(builder.new_dirs()[0]).expect("The dir should be created; qed;");
		builder.new_dirs().iter().enumerate().for_each(|(index, dir_path)| {
			if index == 0 {
				assert!(dir_path.is_dir());
			} else {
				assert!(!dir_path.is_dir());
			}
		});

		// The rollback is executed even if many dirs have not been created yet. That's good
		rollback.rollback_new_dirs();

		// None of the new dirs exists, neither the one which was manually created
		builder.new_dirs().iter().for_each(|dir_path| assert!(!dir_path.is_dir()));
	});
}

#[test]
fn rollback_new_files_works() {
	TestBuilder::new(None).with_new_files().execute(|builder, rollback| {
		// Create one of the new files
		File::create(builder.new_files()[0]).expect("The file should be created; qed;");
		builder.new_files().iter().enumerate().for_each(|(index, file_path)| {
			if index == 0 {
				assert!(file_path.is_file());
			} else {
				assert!(!file_path.is_file());
			}
		});

		// The rollback is executed even if many files have not been created yet. That's good
		rollback.rollback_new_files();

		// None of the new files exists, neither the one which was manually created
		builder.new_files().iter().for_each(|file_path| assert!(!file_path.is_file()));
	});
}

#[test]
fn commit_noted_files_works_well() {
	TestBuilder::new(None).with_noted_files().execute(|builder, rollback| {
		let backups = match rollback.commit_noted_files(Vec::with_capacity(builder.capacity())) {
			Ok(backups) => backups,
			_ => {
				panic!("The call should be Ok");
			},
		};

		// Original paths are committed
		builder.existing_files().iter().for_each(|file| {
			assert_eq!(
				std::fs::read_to_string(file).expect("The file exists"),
				MODIFIED_BUILDER_FILE_CONTENT
			)
		});

		// There's a backup for each noted file
		assert_eq!(backups.len(), builder.capacity());
	});
}

#[test]
fn commit_noted_files_fails_if_a_backup_cannot_be_created() {
	TestBuilder::new(None).with_noted_files().execute(|builder, rollback| {
		// A backup cannot be created if the original has been deleted in the meanwhile
		std::fs::remove_file(builder.existing_files()[0]).expect("The file exists; qed;");
		let (error, backups) =
			match rollback.commit_noted_files(Vec::with_capacity(builder.capacity())) {
				Ok(_) => {
					panic!("The call should be an error");
				},
				Err(output) => output,
			};

		// The error is as expected
		match error {
			Error::Descriptive(msg) => {
				// It says the original file doesn't exist => the backup wasn't created for that
				// file
				assert!(msg.contains(&format!(
					"Committing the following file: {}",
					builder.existing_files()[0].display()
				)));
				assert!(msg.contains("No such file or directory"));
			},
			_ => panic!("Unexpected error"),
		}

		// All the files are committed except the deleted one, there's a backup for every file
		// except for one
		builder.existing_files().iter().enumerate().for_each(|(index, file_path)| {
			if index != 0 {
				assert_eq!(
					std::fs::read_to_string(file_path).expect("The file should be readable; qed;"),
					MODIFIED_BUILDER_FILE_CONTENT
				);
			}
		});

		assert_eq!(backups.len(), builder.capacity() - 1);
	});
}

#[test]
fn commit_noted_files_fails_if_a_noted_file_cannot_be_committed() {
	TestBuilder::new(None).with_noted_files().execute(|builder, rollback| {
		// A file cannot be committed if the temp file was deleted in the meanwhile
		std::fs::remove_file(
			rollback
				.get_noted_file(builder.existing_files()[0])
				.expect("The file is noted, so this is Some; qed;"),
		)
		.expect("The file exists; qed;");

		let (error, backups) =
			match rollback.commit_noted_files(Vec::with_capacity(builder.capacity())) {
				Ok(_) => {
					panic!("The call should be an error");
				},
				Err(output) => output,
			};

		// The error is as expected
		match error {
			Error::Descriptive(msg) => {
				// The original file couldn't be committed
				assert!(msg.contains(&format!(
					"Committing the following file: {}",
					builder.existing_files()[0].display()
				)));
				assert!(msg.contains("No such file or directory"));
			},
			_ => panic!("Unexpected error"),
		}

		// All the files are committed except the one whose temporary file was deleted. There's
		// a backup for every file
		builder.existing_files().iter().enumerate().for_each(|(index, file_path)| {
			if index == 0 {
				assert_eq!(
					std::fs::read_to_string(file_path).expect("The file should be readable; qed;"),
					ORIGINAL_BUILDER_FILE_CONTENT
				);
			} else {
				assert_eq!(
					std::fs::read_to_string(file_path).expect("The file should be readable; qed;"),
					MODIFIED_BUILDER_FILE_CONTENT
				);
			}
		});

		assert_eq!(backups.len(), builder.capacity());
	});
}

#[test]
fn commit_new_dirs_works() {
	TestBuilder::new(None).with_new_dirs().execute(|builder, rollback| {
		builder.new_dirs().iter().for_each(|dir_path| assert!(!dir_path.is_dir()));

		assert!(rollback.commit_new_dirs().is_ok());

		builder.new_dirs().iter().for_each(|dir_path| assert!(dir_path.is_dir()));
	});
}

#[test]
fn commit_new_dirs_fails_if_new_dirs_cannot_be_created() {
	TestBuilder::new(None)
		.with_new_dirs()
		.with_read_only_dir()
		.execute(|builder, rollback| {
			builder.new_dirs().iter().for_each(|dir_path| assert!(!dir_path.is_dir()));

			match rollback.commit_new_dirs() {
				Err(Error::Descriptive(msg)) => {
					// No permissions in temp_dir => failure committing the dirs; Cannot ensure
					// which one comes in the msg cause this runs concurrently and all of them
					// failed
					assert!(msg.contains("Committing the following dir"));
					assert!(msg.contains("Permission denied"));
				},
				_ => panic!("Unexpected error"),
			}

			// Dirs weren't created
			builder.new_dirs().iter().for_each(|dir_path| assert!(!dir_path.is_dir()));
		});
}

#[test]
fn commit_new_dirs_fails_if_same_dir_noted_several_times() {
	TestBuilder::new(None).with_new_dirs().execute(|builder, mut rollback| {
		let path = builder.new_dirs()[0];

		let original_cwd = std::env::current_dir().expect("The current dir is the crate dir; qed;");

		std::env::set_current_dir(builder.get_temp_dir_path())
			.expect("The tempdir should be able to be current_dir; qed;");

		let refactored_path =
			Path::new(path.file_name().expect("The path is a dir, so file_name exists; qed;"));

		assert!(rollback.new_dir(&refactored_path).is_ok());

		let result = rollback.commit_new_dirs();

		std::env::set_current_dir(original_cwd)
			.expect("The original_cwd should be able to be current_dir; qed;");

		assert!(matches!(
			result,
			Err(Error::RepeatedNewDir(msg)) if msg.contains(refactored_path.to_str().unwrap())
		));
	});
}

#[test]
fn commit_new_files_works() {
	TestBuilder::new(None).with_new_files().execute(|builder, rollback| {
		builder.new_files().iter().for_each(|file_path| assert!(!file_path.is_file()));

		assert!(rollback.commit_new_files().is_ok());

		builder.new_files().iter().for_each(|file_path| {
			assert!(file_path.is_file());
			assert_eq!(
				std::fs::read_to_string(file_path).expect("The file exists and is readable; qed;"),
				ORIGINAL_BUILDER_FILE_CONTENT
			);
		});
	});
}

#[test]
fn commit_new_files_fails_if_same_file_noted_several_times() {
	TestBuilder::new(None).with_new_files().execute(|builder, mut rollback| {
		let path = builder.new_files()[0];

		let original_cwd = std::env::current_dir().expect("The current dir is the crate dir; qed;");

		std::env::set_current_dir(builder.get_temp_dir_path())
			.expect("The tempdir should be able to be current_dir; qed;");

		let refactored_path =
			Path::new(path.file_name().expect("The path is a file, so file_name exists; qed;"));

		assert!(rollback.new_file(&refactored_path).is_ok());

		let result = rollback.commit_new_files();

		std::env::set_current_dir(original_cwd)
			.expect("The original_cwd should be able to be current_dir; qed;");

		assert!(matches!(
			result,
			Err(Error::RepeatedNewFile(msg)) if msg.contains(refactored_path.to_str().unwrap())
		));
	});
}

#[test]
fn commit_new_files_fails_if_new_files_cannot_be_created() {
	TestBuilder::new(None)
		.with_new_files()
		.with_read_only_dir()
		.execute(|builder, rollback| {
			builder.new_files().iter().for_each(|file_path| assert!(!file_path.is_file()));

			match rollback.commit_new_files() {
				Err(Error::Descriptive(msg)) => {
					// No permissions in temp_dir => failure committing the files; cannot ensure
					// which one comes in the message as this runs concurrently and all of them
					// failed
					assert!(msg.contains("Committing the following file"));
					assert!(msg.contains("Permission denied"));

					// Files weren't created
					builder.new_files().iter().for_each(|file_path| assert!(!file_path.is_file()));
				},
				_ => panic!("Unexpected error"),
			}
		});
}

#[test]
fn commit_new_files_fails_if_the_temp_file_cannot_be_copied_to_the_new_file() {
	TestBuilder::new(None).with_new_files().execute(|builder, rollback| {
		builder.new_files().iter().for_each(|file_path| assert!(!file_path.is_file()));

		//Remove one of the temporary files
		std::fs::remove_file(
			rollback
				.get_new_file(builder.new_files()[0])
				.expect("The new file is noted by the rollback, so this exists; qed;"),
		)
		.expect("The temporary file can be deleted; qed;");

		match rollback.commit_new_files() {
			Err(Error::Descriptive(msg)) => {
				// The temporary file was deleted for the first new file so it couldn't be
				// created
				assert!(msg.contains(&format!(
					"Committing the following file: {}",
					builder.new_files()[0].display()
				)));
				assert!(msg.contains("No such file or directory"));
			},
			_ => panic!("Unexpected error"),
		}

		// The while the first file is created without content, other files may have been correctly
		// created (maybe not all of them)
		builder.new_files().iter().enumerate().for_each(|(index, file_path)| {
			if index == 0 {
				assert!(file_path.is_file());
				assert_eq!(
					std::fs::read_to_string(file_path)
						.expect("The file exists and is readable; qed;"),
					""
				);
			} else {
				if file_path.is_file() {
					assert_eq!(
						std::fs::read_to_string(file_path)
							.expect("The file exists and is readable; qed;"),
						ORIGINAL_BUILDER_FILE_CONTENT
					);
				}
			}
		});
	});
}
