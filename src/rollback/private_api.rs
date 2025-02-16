// SPDX-License-Identifier: GPL-3.0

#[cfg(all(test, not(feature = "integration-tests")))]
mod tests;

use crate::{
	rollback::{backup::Backup, Rollback},
	Error,
};

use std::{
	fs::File,
	io::ErrorKind,
	sync::{Arc, Mutex},
};

impl Rollback<'_> {
	pub(crate) fn rollback_new_dirs(&self) {
		let mut handles = Vec::with_capacity(self.new_dirs.len());

		for dir in self.new_dirs.iter() {
			let dir = dir.to_path_buf();
			handles.push(std::thread::spawn(move || {
				// Don't need to handle this result:
				// - If it works: ✅
				// - If it fails cause the dir doesn't exist: ✅ as the funcion objective is to
				//   delete it.
				// - By construction there's not other possible error. If the dir is created by the
				//   commit flow, the commit flow can also delete it.
				let _ = std::fs::remove_dir_all(dir);
			}));
		}

		for handle in handles {
			handle.join().expect("The threads cannot panic; qed;");
		}
	}

	pub(crate) fn rollback_new_files(&self) {
		let mut handles = Vec::with_capacity(self.new_files.len());

		for file in self.new_files.keys() {
			let file = file.to_path_buf();
			handles.push(std::thread::spawn(move || {
				// Don't need to handle this result:
				// - If it works: ✅
				// - If it fails cause the file doesn't exist: ✅ as the funcion objective is to
				//   delete it.
				// - By construction there's not other possible error. If the file is created by the
				//   commit flow, the commit flow can also delete it.
				let _ = std::fs::remove_file(file);
			}));
		}

		for handle in handles {
			handle.join().expect("The threads cannot panic; qed;");
		}
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
			let temporal = temporal.path().to_path_buf();
			let mutex_backups = Arc::clone(&mutex_backups);
			handles.push(std::thread::spawn(move || -> Result<(), Error> {
				let backup = match Backup::new(&original) {
					Ok(backup) => backup,
					Err(err) => {
						return Err(Error::Descriptive(format!(
							"Committing the following file: {} failed with error: {}",
							original.display(),
							err
						)));
					},
				};

				let mut backups = mutex_backups.lock().expect("The threads cannot panic; qed;");

				backups.push(backup);

				if let Err(err) = std::fs::copy(temporal, &original) {
					return Err(Error::Descriptive(format!(
						"Committing the following file: {} failed with error: {}",
						original.display(),
						err
					)));
				}
				Ok(())
			}));
		}

		let mut result = Ok(());
		for handle in handles {
			let handle_result = handle.join().expect("The threads cannot panic; qed;");
			if handle_result.is_err() {
				result = handle_result;
			}
		}

		let mut backups = main_thread_backups_copy
			.lock()
			.expect("At this point, this is the only reference to backups still alive and threads cannot panic; qed;");

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
						"Committing the following dir: {} failed with error: {}",
						dir.display(),
						err
					))),
				}
			}));
		}

		let mut result = Ok(());
		for handle in handles {
			let handle_result = handle.join().expect("The threads cannot panic; qed");
			if handle_result.is_err() {
				result = handle_result;
			}
		}

		result
	}

	pub(crate) fn commit_new_files(&self) -> Result<(), Error> {
		let mut handles = Vec::with_capacity(self.new_files.len());
		for (path, temporal) in self.new_files.iter() {
			let path = path.to_path_buf();
			let temporal = temporal.path().to_path_buf();

			handles.push(std::thread::spawn(move || -> Result<(), Error> {
				match File::create(&path) {
					Ok(_) => (),
					Err(err) => {
						return Err(Error::Descriptive(format!(
							"Committing the following file: {} failed with error: {}",
							path.display(),
							err
						)));
					},
				}

				if let Err(err) = std::fs::copy(temporal, &path) {
					return Err(Error::Descriptive(format!(
						"Committing the following file: {} failed with error: {}",
						path.display(),
						err
					)));
				}

				Ok(())
			}));
		}

		let mut result = Ok(());
		for handle in handles {
			let handle_result = handle.join().expect("The threads cannot panic; qed");
			if handle_result.is_err() {
				result = handle_result;
			}
		}

		result
	}
}
