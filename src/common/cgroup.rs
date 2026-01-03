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

use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CGroup(PathBuf);

impl CGroup {
	pub fn current() -> Self {
		Self::from_proc_pid_cgroup(process::id())
	}

	pub fn from_proc_pid_cgroup(pid: u32) -> Self {
		let mut path = PathBuf::from("/proc");
		path.push(pid.to_string());
		path.push("cgroup");
		let file_contents = fs::read_to_string(&path).unwrap();
		let Some(s) = file_contents.trim().strip_prefix("0::") else {
			panic!("Unexpected format in cgroup file. Are you using cgroups v1?\n\n{file_contents}");
		};
		Self(PathBuf::from(s))
	}
}
