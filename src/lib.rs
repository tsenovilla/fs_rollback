// SPDX-License-Identifier: GPL-3.0

//! # Description
//!
//! This crate introduces the Rollback struct, a whole rollback mechanism for file system
//! transactions. Rollback allows to atomically create/modify files and to create directories, so
//! the file system won't be affected by these changes unless the Rollback instance is committed.  
//!
//! # Examples
//!
//! A successful execution of a program using the Rollback struct may look like the code below. As
//! it's shown in the example, all the files and directories are successfully committed.
//!
//! ```
//! use fs_rollback::Rollback;
//! use std::fs::File;
//!     
//! let tempdir = tempfile::tempdir().unwrap();
//!
//! // Some file that already exists
//! let existing_file = tempdir.path().join("file.txt");
//! File::create(&existing_file).unwrap();
//! std::fs::write(&existing_file, "Hello world!").unwrap();
//! assert_eq!("Hello world!", std::fs::read_to_string(&existing_file).unwrap());
//!
//! // Some dirs that don't exist yet
//! let dir1 = tempdir.path().join("dir1");
//! let dir2 = dir1.join("dir2");
//! let dir3 = tempdir.path().join("dir3");
//! assert!(!dir1.is_dir());
//! assert!(!dir2.is_dir());
//! assert!(!dir3.is_dir());
//!
//! // Some files that don't exist yet, even if they're contained in one of the non-existing dirs above
//! let new_file1 = dir2.join("file1.txt");
//! let new_file2 = tempdir.path().join("file2.txt");
//! assert!(!new_file1.is_file());
//! assert!(!new_file2.is_file());
//!
//! // Rollback instance with capacity for the needed paths
//! let mut rollback = Rollback::with_capacity(1,2,3);
//! rollback.note_file(&existing_file).unwrap();
//! rollback.new_file(&new_file1).unwrap();
//! rollback.new_file(&new_file2).unwrap();
//! rollback.new_dir(&dir1).unwrap();
//! rollback.new_dir(&dir2).unwrap();
//! rollback.new_dir(&dir3).unwrap();
//!
//! // Some operations with the new files and the noted files.
//! std::fs::write(rollback.get_noted_file(&existing_file).unwrap(),"Happy to commit this").unwrap();
//! std::fs::write(rollback.get_new_file(&new_file1).unwrap(),"Happy to commit this").unwrap();
//!
//! // If everything went well, we can commit our changes to the fs
//! rollback.commit().unwrap();
//!
//! // And everything's commited!
//! assert!(dir1.is_dir());
//! assert!(dir2.is_dir());
//! assert!(dir3.is_dir());
//! assert!(new_file1.is_file());
//! assert!(new_file2.is_file());
//! assert_eq!("Happy to commit this", std::fs::read_to_string(&existing_file).unwrap());
//! assert_eq!("Happy to commit this", std::fs::read_to_string(&new_file1).unwrap());
//! ```
//!
//! There's plenty of reasons leading to a failing execution, all of them resulting in an unaltered
//! file system. Visit the Rollback struct docs to learn about all the available methods, as well
//! as their possible errors.
//!
//! This example shows that a failed commit rollbacks everything.
//!
//! ```
//! use fs_rollback::{Rollback, Error};
//! use std::fs::File;
//!     
//! let tempdir = tempfile::tempdir().unwrap();
//!
//! // Some file that already exists
//! let existing_file = tempdir.path().join("file.txt");
//! File::create(&existing_file).unwrap();
//! std::fs::write(&existing_file, "Hello world!").unwrap();
//! assert_eq!("Hello world!", std::fs::read_to_string(&existing_file).unwrap());
//!
//! // Some dirs that don't exist yet
//! let dir1 = tempdir.path().join("dir1");
//! let dir2 = dir1.join("dir2");
//! assert!(!dir1.is_dir());
//! assert!(!dir2.is_dir());
//!
//! // A file that doesn't exist and that cannot be committed due to its parent dir doesn't exist
//! // and that won't be noted by the rollback. This file will cause that the rollback commit
//! // fails.
//! let new_file1 = dir2.join("file1.txt");
//! assert!(!new_file1.is_file());
//!
//! // Rollback instance with capacity for the needed paths
//! let mut rollback = Rollback::with_capacity(1,1,2);
//! rollback.note_file(&existing_file).unwrap();
//! rollback.new_file(&new_file1).unwrap();
//! rollback.new_dir(&dir1).unwrap();
//!
//! // Some operations with the new files and the noted files.
//! std::fs::write(rollback.get_noted_file(&existing_file).unwrap(),"Happy to commit this").unwrap();
//! std::fs::write(rollback.get_new_file(&new_file1).unwrap(),"Happy to commit this").unwrap();
//!
//! // If everything went well, we can commit our changes to the fs
//! match rollback.commit(){
//!     Err(Error::Descriptive(msg)) => {
//!         // The error specifies the uncommited file
//!         assert!(msg.contains(&format!("Committing the following file: {}",new_file1.display())));
//!         // As the error's originated by a not existing directory, the message also explains
//!         // that
//!         assert!(msg.contains("No such file or directory"));
//!     },
//!     _ => panic!("Unexpected error")
//! }
//!
//! // And everything's rolled back!
//! assert!(!dir1.is_dir());
//! assert!(!dir2.is_dir());
//! assert!(!new_file1.is_file());
//! assert_eq!("Hello world!", std::fs::read_to_string(&existing_file).unwrap());
//! ```
//!
//! While this example shows that uncommitted changes are just discarded if the Rollback instance
//! goes out of scope.
//!
//! ```
//! use fs_rollback::Rollback;
//! use std::path::PathBuf;
//!
//! let tempdir = tempfile::tempdir().unwrap();
//! let new_file = tempdir.path().join("file.txt");
//! let mut tempfile = PathBuf::new();
//! assert!(!new_file.is_file());
//! assert!(!tempfile.is_file());
//!
//! {
//!     let mut rollback = Rollback::default();
//!     rollback.new_file(&new_file).unwrap();
//!     tempfile = rollback.get_new_file(&new_file).unwrap().to_path_buf();
//!     assert!(tempfile.is_file());
//!     std::fs::write(&tempfile,"Hello world!").unwrap();
//! }
//!
//! assert!(!new_file.is_file());
//! // tempfile contains a path to the temporary file created by the rollback, but as the rollback
//! // went out of scope, that path doesn't point anymore to an existing file. That file was
//! // discarded.
//! assert!(!tempfile.is_file() && tempfile != PathBuf::new());
//! ```

mod error;
mod rollback;
#[cfg(any(test, feature = "integration-tests"))]
pub mod test_builder;

pub use error::Error;
pub use rollback::Rollback;
