// SPDX-License-Identifier: GPL-3.0

#[cfg(test)]
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
		let backup = NamedTempFile::new()?;
		std::fs::copy(original, &backup)?;
		Ok(Self { backup, original: original.to_path_buf() })
	}

	pub(crate) fn rollback(&self) {
		std::fs::copy(
			&self.backup,
			&self.original
		)
		.expect("Generated backups guarantee that both original and backup exist and are writable; qed;");
	}
}
