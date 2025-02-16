// SPDX-License-Identifier: GPL-3.0

use super::*;

#[test]
fn rollback_new_works() {
	let rollback = Rollback::default();

	assert!(rollback.noted.is_empty() && rollback.noted.capacity() == 0);
	assert!(rollback.new_files.is_empty() && rollback.new_files.capacity() == 0);
	assert!(rollback.new_dirs.is_empty() && rollback.new_dirs.capacity() == 0);
}

#[test]
fn rollback_with_capacity_works() {
	let rollback = Rollback::with_capacity(1, 2, 3);

	assert!(rollback.noted.is_empty() && rollback.noted.capacity() >= 1);
	assert!(rollback.new_files.is_empty() && rollback.new_files.capacity() >= 2);
	assert!(rollback.new_dirs.is_empty() && rollback.new_dirs.capacity() == 3);
}
