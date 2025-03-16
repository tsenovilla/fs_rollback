// SPDX-License-Identifier: GPL-3.0

#![cfg(feature = "integration-tests")]

use fs_rollback::{
	test_builder::{TestBuilder, MODIFIED_BUILDER_FILE_CONTENT, ORIGINAL_BUILDER_FILE_CONTENT},
	Error,
};
use std::{fs::File, io::ErrorKind, path::Path};

#[test]
fn note_file_works() {
	TestBuilder::new(Some(1)).execute(|builder, mut rollback| {
		let path = builder.existing_files()[0];
		assert!(rollback.note_file(path).is_ok());
		let roll_file = rollback
			.get_noted_file(path)
			.expect("The file should be correctly noted in the rollback; qed;");

		// The new file is a copy of the original one
		assert_eq!(
			std::fs::read_to_string(path).expect("The file exists; qed;"),
			std::fs::read_to_string(roll_file).expect("The file exists; qed;")
		);
	});
}

#[test]
fn note_file_fails_if_provided_path_isnt_file() {
	TestBuilder::new(Some(0)).execute(|_, mut rollback| {
		let some_path = "some/path";

		match rollback.note_file(some_path.as_ref()) {
			Err(Error::Descriptive(msg)) => assert_eq!(msg, format!("{} isn't a file.", some_path)),
			_ => panic!("Unexpected error"),
		}
	});
}

#[test]
fn note_file_fails_if_provided_path_is_already_noted() {
	TestBuilder::new(Some(1)).with_noted_files().execute(|builder, mut rollback| {
		let path = builder.existing_files()[0];
		match rollback.note_file(path) {
			Err(Error::Descriptive(msg)) =>
				assert_eq!(msg, format!("{} is already noted by this rollback.", path.display())),
			_ => panic!("Unexpected error"),
		}
	});
}

#[test]
fn note_file_fails_if_it_cannot_create_temp_file() {
	// Save original tempdir locations as this test will modify them.
	let original_temp_dir = std::env::var("TEMP").unwrap_or_default(); // Windows
	let original_tmpdir = std::env::var("TMPDIR").unwrap_or_default(); // UNIX

	TestBuilder::new(Some(1))
		.with_read_only_temp_dir()
		.execute(|builder, mut rollback| {
			let result = rollback.note_file(builder.existing_files()[0]);

			// Reset original tempdir locations before asserting, so we're safe in case of panic
			std::env::set_var("TMPDIR", original_tmpdir.clone());
			std::env::set_var("TEMP", original_temp_dir.clone());

			match result {
				Err(Error::IO(err)) => assert_eq!(err.kind(), ErrorKind::PermissionDenied),
				_ => panic!("Unexpected error"),
			}
		});
}

#[test]
fn note_file_fails_if_original_file_cannot_be_copied() {
	TestBuilder::new(Some(1))
		.with_permissionless_files()
		.execute(|builder, mut rollback| match rollback.note_file(builder.existing_files()[0]) {
			Err(Error::IO(err)) => assert_eq!(err.kind(), ErrorKind::PermissionDenied),
			_ => panic!("Unexpected error"),
		});
}

#[test]
fn new_file_works() {
	TestBuilder::new(Some(1)).execute(|builder, mut rollback| {
		let path = builder.new_files()[0];
		assert!(rollback.new_file(path).is_ok());
		assert!(rollback.get_new_file(path).is_some());
	});
}

#[test]
fn new_file_fails_if_path_already_exists() {
	TestBuilder::new(Some(1)).execute(|builder, mut rollback| {
		let path = builder.existing_files()[0];
		match rollback.new_file(path) {
			Err(Error::Descriptive(msg)) =>
				assert_eq!(format!("{} already exists.", path.display()), msg),
			_ => panic!("Unexpected error"),
		}
	});
}

#[test]
fn new_file_fails_if_path_already_noted() {
	TestBuilder::new(Some(1)).with_new_files().execute(|builder, mut rollback| {
		let path = builder.new_files()[0];
		match rollback.new_file(path) {
			Err(Error::Descriptive(msg)) =>
				assert_eq!(format!("{} is already noted by this rollback.", path.display()), msg),
			_ => panic!("Unexpected error"),
		}
	});
}

#[test]
fn new_file_fails_if_path_cannot_be_a_file() {
	TestBuilder::new(Some(1)).execute(|builder, mut rollback| {
		let path = builder.new_dirs()[0];
		match rollback.new_file(path) {
			Err(Error::Descriptive(msg)) =>
				assert_eq!(format!("{} is supposed to be a valid file path.", path.display()), msg),
			_ => panic!("Unexpected error"),
		}
	});
}

#[test]
fn new_file_fails_if_it_cannot_create_temp_file() {
	// Save original tempdir locations as this test will modify them.
	let original_temp_dir = std::env::var("TEMP").unwrap_or_default(); // Windows
	let original_tmpdir = std::env::var("TMPDIR").unwrap_or_default(); // UNIX

	TestBuilder::new(Some(1))
		.with_read_only_temp_dir()
		.execute(|builder, mut rollback| {
			let result = rollback.new_file(builder.new_files()[0]);

			// Reset original tempdir locations before asserting, so we're safe in case of panic
			std::env::set_var("TMPDIR", original_tmpdir.clone());
			std::env::set_var("TEMP", original_temp_dir.clone());

			match result {
				Err(Error::IO(err)) => assert_eq!(err.kind(), ErrorKind::PermissionDenied),
				_ => panic!("Unexpected error"),
			}
		});
}

#[test]
fn new_dir_works() {
	TestBuilder::new(Some(1)).execute(|builder, mut rollback| {
		let path = builder.new_dirs()[0];
		assert!(rollback.new_dir(path).is_ok());
	});
}

#[test]
fn new_dir_fails_if_path_already_exists() {
	TestBuilder::new(Some(1)).execute(|builder, mut rollback| {
		let path = builder.new_dirs()[0];
		std::fs::create_dir_all(path).expect("The directory should be created; qed;");
		match rollback.new_dir(path) {
			Err(Error::Descriptive(msg)) =>
				assert_eq!(format!("{} already exists.", path.display()), msg),
			_ => panic!("Unexpected error"),
		}
	});
}

#[test]
fn new_dir_fails_if_path_already_noted() {
	TestBuilder::new(Some(1)).with_new_dirs().execute(|builder, mut rollback| {
		let path = builder.new_dirs()[0];
		match rollback.new_dir(path) {
			Err(Error::Descriptive(msg)) =>
				assert_eq!(format!("{} is already noted by this rollback.", path.display()), msg),
			_ => panic!("Unexpected error"),
		}
	});
}

#[test]
fn new_dir_fails_if_path_cannot_be_a_dir() {
	TestBuilder::new(Some(1)).execute(|builder, mut rollback| {
		let path = builder.new_files()[0];
		match rollback.new_dir(path) {
			Err(Error::Descriptive(msg)) => assert_eq!(
				format!("{} is supposed to be a valid directory path.", path.display()),
				msg
			),
			_ => panic!("Unexpected error"),
		}
	});
}

#[test]
fn get_noted_file_works() {
	TestBuilder::new(Some(1)).with_noted_files().execute(|builder, rollback| {
		assert!(rollback.get_noted_file(builder.existing_files()[0]).is_some());
		assert!(rollback.get_noted_file::<&Path>("something".as_ref()).is_none());
	});
}

#[test]
fn get_new_file_works() {
	TestBuilder::new(Some(1)).with_new_files().execute(|builder, rollback| {
		assert!(rollback.get_new_file(builder.new_files()[0]).is_some());
		assert!(rollback.get_new_file::<&Path>("something".as_ref()).is_none());
	});
}

#[test]
fn commit_works() {
	TestBuilder::new(None)
		.with_noted_files()
		.with_new_files()
		.with_new_dirs()
		.execute(|builder, rollback| {
			builder.existing_files().iter().for_each(|file| {
				assert_eq!(
					std::fs::read_to_string(file).expect("The file should be readable; qed;"),
					ORIGINAL_BUILDER_FILE_CONTENT
				)
			});

			builder.new_files().iter().for_each(|file| assert!(!file.is_file()));
			builder.new_dirs().iter().for_each(|dir| assert!(!dir.is_dir()));

			assert!(rollback.commit().is_ok());

			builder.existing_files().iter().for_each(|file| {
				assert_eq!(
					std::fs::read_to_string(file).expect("The file should be readable; qed;"),
					MODIFIED_BUILDER_FILE_CONTENT
				)
			});

			builder.new_files().iter().for_each(|file| {
				assert!(file.is_file());
				assert_eq!(
					std::fs::read_to_string(file).expect("The file should be readable; qed;"),
					ORIGINAL_BUILDER_FILE_CONTENT
				)
			});
			builder.new_dirs().iter().for_each(|dir| assert!(dir.is_dir()));
		});
}

#[test]
fn commit_fails_and_rollbacks_if_noted_file_cannot_be_committed() {
	TestBuilder::new(None)
		.with_noted_files()
		.with_new_files()
		.with_new_dirs()
		.execute(|builder, rollback| {
			builder.existing_files().iter().for_each(|file| {
				assert_eq!(
					std::fs::read_to_string(file).expect("The file should be readable; qed;"),
					ORIGINAL_BUILDER_FILE_CONTENT
				)
			});

			builder.new_files().iter().for_each(|file| assert!(!file.is_file()));
			builder.new_dirs().iter().for_each(|dir| assert!(!dir.is_dir()));

			// Deleting an existing file means that rollback cannot commit that file.
			let removed_file = builder
				.existing_files()
				.last()
				.expect("There's existing_files; qed;")
				.to_path_buf();
			std::fs::remove_file(&removed_file).expect("This should be possible; qed;");

			match rollback.commit() {
				Err(Error::Descriptive(msg)) => {
					assert!(msg.contains(&format!(
						"Committing the following file: {}",
						removed_file.display()
					)));
					assert!(msg.contains("No such file or directory"));
				},
				_ => panic!("Unexpected error"),
			}

			// The fs wasn't affected
			builder.existing_files().iter().enumerate().for_each(|(index, file)| {
				if index < builder.capacity() - 1 {
					assert_eq!(
						std::fs::read_to_string(file).expect("The file should be readable; qed;"),
						ORIGINAL_BUILDER_FILE_CONTENT
					)
				}
			});

			builder.new_files().iter().for_each(|file| assert!(!file.is_file()));
			builder.new_dirs().iter().for_each(|dir| assert!(!dir.is_dir()));
		});
}

#[test]
fn commit_fails_and_rollbacks_if_new_dir_cannot_be_committed() {
	TestBuilder::new(None)
		.with_new_files()
		.with_new_dirs()
		.with_read_only_dir()
		.execute(|builder, rollback| {
			// Note a file (cannot build it with builder cause the commit will fail because of this
			// file as it's in a read_only dir). Rebind rollback to accomplish with existing_file
			// lifetime
			let mut rollback = rollback;
			let tempdir = tempfile::tempdir().expect("Tempdir should be created");
			let existing_file = tempdir.path().join("file.txt");

			File::create(&existing_file).expect("File should be created; qed;");
			std::fs::write(&existing_file, ORIGINAL_BUILDER_FILE_CONTENT)
				.expect("File should be writable; qed;");
			rollback.note_file(&existing_file).expect("File should be noted; qed;");
			builder.new_files().iter().for_each(|file| assert!(!file.is_file()));
			builder.new_dirs().iter().for_each(|dir| assert!(!dir.is_dir()));

			match rollback.commit() {
				Err(Error::Descriptive(msg)) => {
					// No permissions in temp_dir => failure committing the dirs; Cannot ensure
					// which one comes in the msg cause this runs concurrently and all of them
					// failed
					assert!(msg.contains("Committing the following dir"));
					assert!(msg.contains("Permission denied"));
				},
				_ => panic!("Unexpected error"),
			}

			// The fs wasn't affected
			assert_eq!(
				std::fs::read_to_string(&existing_file).expect("File should be readable; qed;"),
				ORIGINAL_BUILDER_FILE_CONTENT
			);
			builder.new_files().iter().for_each(|file| assert!(!file.is_file()));
			builder.new_dirs().iter().for_each(|dir| assert!(!dir.is_dir()));
		});
}

#[test]
fn commit_fails_and_rollbacks_if_new_file_cannot_be_committed() {
	TestBuilder::new(None)
		.with_noted_files()
		.with_new_files()
		.with_new_dirs()
		.execute(|builder, rollback| {
			builder.existing_files().iter().for_each(|file| {
				assert_eq!(
					std::fs::read_to_string(file).expect("The file should be readable; qed;"),
					ORIGINAL_BUILDER_FILE_CONTENT
				)
			});

			builder.new_files().iter().for_each(|file| assert!(!file.is_file()));
			builder.new_dirs().iter().for_each(|dir| assert!(!dir.is_dir()));

			// Deleting a temporary file means that rollback cannot commit the related new file.
			let uncommitted_file =
				builder.new_files().last().expect("There's new_files; qed;").to_path_buf();

			std::fs::remove_file(
				rollback
					.get_new_file(&uncommitted_file)
					.expect("The file exists for this rollback; qed;"),
			)
			.expect("This should be possible; qed;");

			match rollback.commit() {
				Err(Error::Descriptive(msg)) => {
					assert!(msg.contains(&format!(
						"Committing the following file: {}",
						uncommitted_file.display()
					)));
					assert!(msg.contains("No such file or directory"));
				},
				_ => panic!("Unexpected error"),
			}

			// The fs wasn't affected
			builder.existing_files().iter().for_each(|file| {
				assert_eq!(
					std::fs::read_to_string(file).expect("The file should be readable; qed;"),
					ORIGINAL_BUILDER_FILE_CONTENT
				)
			});

			builder.new_files().iter().for_each(|file| assert!(!file.is_file()));
			builder.new_dirs().iter().for_each(|dir| assert!(!dir.is_dir()));
		});
}
