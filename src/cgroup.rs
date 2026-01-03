// Copyright 2026 Octave Online LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process;

/// A control group that may or may not exist on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CGroup(PathBuf);

impl CGroup {
	/// Reads the control group of the current process and returns it.
	pub fn current() -> Self {
		Self::from_proc_pid_cgroup(process::id())
	}

	/// Reads the control group of the given process ID and returns it.
	pub fn from_proc_pid_cgroup(pid: u32) -> Self {
		let mut path = PathBuf::from("/proc");
		path.push(pid.to_string());
		path.push("cgroup");
		let file_contents = fs::read_to_string(&path).unwrap();
		let Some(s) = file_contents.trim().strip_prefix("0::") else {
			panic!("Error: Unexpected format in cgroup file. Are you using cgroups v1?\n\n{file_contents}");
		};
		Self(PathBuf::from(s))
	}

	/// Creates a [`CGroup`] from a path relative to the cgroup file system.
	pub fn from_cgroup_path(path: impl AsRef<Path>) -> Self {
		Self(PathBuf::from(path.as_ref()))
	}

	/// Returns this [`CGroup`] as a path relative to the cgroup file system.
	pub fn as_cgroup_path(&self) -> &Path {
		&self.0
	}

	/// Returns true if the cgroup was modified.
	///
	/// # Examples
	///
	/// ```
	/// use cg2tools::CGroup;
	///
	/// let mut cgroup = CGroup::from_cgroup_path("/a/b/c");
	/// assert_eq!(cgroup.append("d"), true);
	/// assert_eq!(cgroup.as_cgroup_path().to_str(), Some("/a/b/c/d"));
	/// assert_eq!(cgroup.append("/e"), true);
	/// assert_eq!(cgroup.as_cgroup_path().to_str(), Some("/e"));
	/// assert_eq!(cgroup.append("/e"), false);
	/// assert_eq!(cgroup.as_cgroup_path().to_str(), Some("/e"));
	/// ```
	pub fn append(&mut self, path: impl AsRef<Path>) -> bool {
		let new_path = self.0.join(path);
		if self.0 == new_path {
			return false;
		}
		self.0 = new_path;
		true
	}

	/// Returns the parent of this [`CGroup`] if there is one.
	pub fn parent(&self) -> Option<Self> {
		self.0.parent().map(Path::to_path_buf).map(Self)
	}

	fn cgroupfs_path(&self) -> PathBuf {
		Path::new("/sys/fs/cgroup").join(&self.0.strip_prefix("/").unwrap())
	}

	fn cgroupfs_path_if_exists(&self) -> Option<PathBuf> {
		let path = self.cgroupfs_path();
		path.try_exists().unwrap().then_some(path)
	}

	/// Creates the CGroup on the filesystem if it doesn't exist yet.
	///
	/// If newly created, also sets the owner.
	pub fn create(&self) {
		let path = self.cgroupfs_path();
		let exists = path.try_exists().unwrap();
		if exists {
			println!("Notice: Control group {self} already exists");
			return;
		}
		match fs::create_dir_all(&path) {
			Ok(()) => (),
			Err(e) => panic!("Error: While creating control group {self}: {e}"),
		}
		println!("Notice: Created control group {self}");
	}

	/// Classifies the given process ID into this [`CGroup`].
	pub fn classify(&self, pid: u32) {
		let Some(mut path) = self.cgroupfs_path_if_exists() else {
			panic!("Error: Control group {self} does not exist");
		};
		path.push("cgroup.procs");
		let mut f = match File::options().append(true).open(&path) {
			Ok(f) => f,
			Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
				panic!("Error: Permission denied: cannot assign to control group {self}");
			}
			Err(e) => panic!("Error: While assigning {pid} to control group {self}: {e}"),
		};
		match write!(&mut f, "{}", pid) {
			Ok(()) => (),
			Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
				panic!("Error: Permission denied: cannot detach process from existing cgroup");
			}
			Err(e) => panic!("Error: While assigning {pid} to control group {self}: {e}"),
		}
	}

	/// Classifies the current process into this [`CGroup`].
	pub fn classify_current(&self) {
		self.classify(process::id())
	}

	/// Loads the controllers enabled for this [`CGroup`].
	pub fn controllers(&self) -> Vec<String> {
		let Some(mut path) = self.cgroupfs_path_if_exists() else {
			panic!("Error: Control group {self} does not exist");
		};
		path.push("cgroup.controllers");
		let mut f = match File::options().read(true).open(&path) {
			Ok(f) => f,
			Err(e) => panic!("Error: While loading the controllers of {self}: {e}"),
		};
		let mut contents = String::new();
		f.read_to_string(&mut contents).unwrap();
		contents.trim().split_whitespace().map(ToString::to_string).collect()
	}

	pub fn has_processes(&self) -> bool {
		let Some(mut path) = self.cgroupfs_path_if_exists() else {
			panic!("Error: Control group {self} does not exist");
		};
		path.push("cgroup.procs");
		let mut f = match File::options().read(true).open(&path) {
			Ok(f) => f,
			Err(e) => panic!("Error: While loading the processes of {self}: {e}"),
		};
		let mut contents = String::new();
		f.read_to_string(&mut contents).unwrap();
		!contents.trim().is_empty()
	}

	/// Allow children of the current [`CGroup`] to set restrictions on the given controllers.
	pub fn enable_subtree_control(&self, new_controllers: &[&str]) {
		if self.has_processes() {
			println!("Warning: Control group {self} owns one or more processes. Enabling controllers in children of nonempty control groups can cause unexpected behavior. For example, a domain cgroup might turned into a threaded domain. See <https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html>")
		}
		self.enable_controllers(new_controllers);
		let Some(mut path) = self.cgroupfs_path_if_exists() else {
			panic!("Error: Control group {self} does not exist");
		};
		path.push("cgroup.subtree_control");
		let mut f = match File::options().append(true).open(&path) {
			Ok(f) => f,
			Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
				panic!("Error: Permission denied: cannot change cgroup.subtree_control for control group {self}");
			}
			Err(e) => panic!("Error: Opening {path:?}: {e}"),
		};
		for controller in new_controllers {
			// It seems that this needs to be written as one chunk
			let str_to_write = format!("+{controller}");
			match write!(&mut f, "{str_to_write}") {
				Ok(()) => {
					println!("Notice: Enabled controller \"{controller}\" for subgroups of {self}");
				}
				Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
					panic!(
						"Error: Permission denied: cannot enable controller \"{controller}\" in control group {self}"
					);
				}
				Err(e) => panic!("Error: Writing to {path:?}: {e}"),
			}
		}
	}

	/// Allow the current [`CGroup`] to set restrictions on the given controllers.
	pub fn enable_controllers(&self, new_controllers: &[&str]) {
		let current_controllers = self.controllers();
		let needed_controllers = new_controllers
			.iter()
			.filter(|c| !current_controllers.iter().any(|x| &x == c))
			.copied()
			.collect::<Vec<_>>();
		if needed_controllers.is_empty() {
			// Nothing to do
			return;
		}
		let Some(parent) = self.parent() else {
			panic!("Error: Some controllers are not available on this system: {needed_controllers:?}");
		};
		parent.enable_subtree_control(needed_controllers.as_slice());
	}

	/// Sets a restriction based on the key (file name, like "cpu.max") and value (like "90000 100000").
	///
	/// See <https://docs.kernel.org/admin-guide/cgroup-v2.html>
	pub fn set_restriction(&self, key: &str, value: &str) {
		let Some(mut path) = self.cgroupfs_path_if_exists() else {
			panic!("Error: Control group {self} does not exist");
		};
		path.push(key);
		let mut f = match File::options().write(true).open(&path) {
			Ok(f) => f,
			Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
				panic!("Error: Permission denied: cannot set restriction {key} in control group {self}");
			}
			Err(e) if e.kind() == io::ErrorKind::NotFound => {
				panic!("Error: Restriction {key} is unavailable for control group {self}");
			}
			Err(e) => panic!("Error: {e}"),
		};
		match write!(&mut f, "{}", value) {
			Ok(()) => {
				println!("Notice: Restriction {key}=\"{value}\" set in control group {self}");
			}
			Err(e) => panic!("Error: While writing to {path:?}: {e}"),
		}
	}
}

impl AsRef<Path> for CGroup {
	fn as_ref(&self) -> &Path {
		&self.0
	}
}

impl fmt::Display for CGroup {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
		self.0.display().fmt(f)
	}
}
