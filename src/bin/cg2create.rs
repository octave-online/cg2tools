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

use cg2tools::internal;
use cg2tools::CGroup;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about = "Runs a program with a specific control group")]
struct Args {
	/// Name of the control group. May be relative (appended to the control group of the current process) or absolute (starting with "/").
	#[arg(short = 'g', long)]
	cgroup: String,

	/// Owner of the new control group, only if the group is newly created.
	#[arg(short = 'u', long)]
	user: Option<String>,
}

fn main() {
	let args = Args::parse();
	internal::os_check();
	let mut cgroup = CGroup::current();
	cgroup.append(&args.cgroup);
	cgroup.create_and_chown(args.user.as_deref());
}
