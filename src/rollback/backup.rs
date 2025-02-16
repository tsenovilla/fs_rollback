// SPDX-License-Identifier: GPL-3.0

#[cfg(all(test, not(feature = "integration-tests")))]
mod tests;

use crate::Error;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

// A useful struct to create temporary backups and rollback them if needed
pub(crate) struct Backup {
	backup: NamedTempFile,
	original: PathBuf,
}

impl Backup {
	pub(crate) fn new(original: &Path) -> Result<Self, Error> {
		let prefixed_path = rustilities::paths::prefix_with_current_dir(original);
		let original_parent_dir =
			prefixed_path.parent().expect("The path is a file and is prefixed; qed;");
		// Create the backup in the same directory as the original, so we can persist the backup
		let backup = NamedTempFile::new_in(original_parent_dir)?;
		std::fs::copy(original, &backup)?;
		Ok(Self { backup, original: original.to_path_buf() })
	}

	pub(crate) fn rollback(self) {
		self.backup
            .persist(&self.original)
            .expect("Generated backups guarantee that both original and backup exist in the same file system, so persisting the tempfile should be possible; qed;");
	}
}
