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

//! This package contains lightweight CLI tools for manipulating Unified Control Groups (also known as cgroups v2) in the Linux kernel via the cgroupfs.
//!
//! The tools are primarily designed for services with `Delegate=yes` in their [systemd configuration](https://systemd.io/CGROUP_DELEGATION/).
//!
//! There are currently two tools:
//!
//! - `cg2exec` for running subcommands in specific cgroups.
//! - `cg2util` for configuring cgroups and classifying existing processes.
//!
//! For more information, see [the project README](https://github.com/octave-online/cg2tools?tab=readme-ov-file#cg2tools).

mod cgroup;

#[doc(hidden)]
pub mod internal;

pub use cgroup::CGroup;
