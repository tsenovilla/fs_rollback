// SPDX-License-Identifier: GPL-3.0

use crate::{
	rollback::{backup::Backup, Rollback},
	Error,
};
use std::{
	fs::File,
	io::ErrorKind,
	sync::{Arc, Mutex},
};

impl<'a> Rollback<'a> {
	pub(crate) fn rollback_new_dirs(&self) {
		self.new_dirs.iter().for_each(|dir| match std::fs::remove_dir_all(dir) {
			Err(err) => match Error::from(err) {
				// This only means that this directory isn't created, it's OK, we don't need to
				// remove it
				Error::IO(err) if err.kind() != ErrorKind::NotFound => (),
				// By construction, this dir has been created by the flow, so the flow can remove it
				_ => unreachable!(),
			},
			_ => (),
		});
	}

	pub(crate) fn rollback_new_files(&self) {
		self.new_files.keys().for_each(|file| match std::fs::remove_file(file) {
			Err(err) => match Error::from(err) {
				// This only means that this file isn't created yet, it's OK, don't need to remove
				// it
				Error::IO(err) if err.kind() != ErrorKind::NotFound => (),
				// By construction, this file has been created by the flow, so the flow can remove
				// it
				_ => unreachable!(),
			},
			_ => (),
		});
	}

	pub(crate) fn commit_noted_files(
		&self,
		backups: Vec<Backup>,
	) -> Result<Vec<Backup>, (Error, Vec<Backup>)> {
		let mut handles = Vec::with_capacity(self.noted.len());

		let mutex_backups = Arc::new(Mutex::new(backups));

		// Keep an Arc count in the main thread as we need to send back backups at the end of the
		// process
		let main_thread_backups_copy = Arc::clone(&mutex_backups);

		// Keep track of all successfully created backups and return an error if something goes
		// wrong in any thread.
		for (original, temporal) in self.noted.iter() {
			let original = original.to_path_buf();
			let temporal = temporal.clone();
			let mutex_backups = Arc::clone(&mutex_backups);
			handles.push(std::thread::spawn(move || -> Result<(), Error> {
				let backup = match Backup::new(&original) {
					Ok(backup) => backup,
					Err(err) => {
						return Err(Error::Descriptive(format!(
							"Committing the following file: {:?} failed with error: {}",
							original, err
						)));
					},
				};

				let mut backups = mutex_backups.lock().map_err(|err| {
					Error::Descriptive(format!(
						"Committing the following file: {:?} failed with error: {}",
						original, err
					))
				})?;

				backups.push(backup);

				if let Err(err) = std::fs::copy(temporal, &original) {
					return Err(Error::Descriptive(format!(
						"Committing the following file: {:?} failed with error: {}",
						original, err
					)));
				}
				Ok(())
			}));
		}

		let mut result = Ok(());
		for handle in handles {
			let handle_result = handle.join().expect("The threads cannot panic; qed;");
			if let Err(err) = handle_result {
				result = Err(err);
			}
		}

		let mut backups = main_thread_backups_copy
			.lock()
			.expect("At this point, this is the only reference to backups still alive; qed;");

		// The only way to get backups back is to take its memory from the MutexGuard. It's fine, as
		// there's not any remaining threads that can access this data
		match result {
			Ok(_) => Ok(std::mem::take(&mut *backups)),
			Err(err) => Err((err, std::mem::take(&mut *backups))),
		}
	}

	pub(crate) fn commit_new_dirs(&self) -> Result<(), Error> {
		let mut handles = Vec::with_capacity(self.new_dirs.len());
		for dir in self.new_dirs.iter() {
			let dir = dir.to_path_buf();
			handles.push(std::thread::spawn(move || -> Result<(), Error> {
				match std::fs::create_dir_all(&dir) {
					Ok(_) => Ok(()),
					Err(err) => Err(Error::Descriptive(format!(
						"Committing the following dir: {:?} failed with error: {}",
						dir, err
					))),
				}
			}));
		}

		for handle in handles {
			let result = handle.join().expect("The threads cannot panic; qed;");
			if result.is_err() {
				return result;
			}
		}
		Ok(())
	}

	pub(crate) fn commit_new_files(&self) -> Result<(), Error> {
		let mut handles = Vec::with_capacity(self.new_files.len());
		for (path, temporal) in self.new_files.iter() {
			let path = path.to_path_buf();
			let temporal = temporal.clone();

			handles.push(std::thread::spawn(move || -> Result<(), Error> {
				match File::create(&path) {
					Ok(_) => (),
					Err(err) => {
						return Err(Error::Descriptive(format!(
							"Committing the following file: {:?} failed with error: {}",
							path, err
						)));
					},
				}

				if let Err(err) = std::fs::copy(temporal, &path) {
					return Err(Error::Descriptive(format!(
						"Committing the following file: {:?} failed with error: {}",
						path, err
					)));
				}

				Ok(())
			}));
		}

		for handle in handles {
			let result = handle.join().expect("The threads cannot panic; qed");

			if result.is_err() {
				return result;
			}
		}
		Ok(())
	}
}
