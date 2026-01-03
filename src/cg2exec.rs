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

use cg2tools::common;
use cg2tools::common::CGroup;
use clap::Parser;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(version, about = "Runs a program with a specific control group")]
struct Args {
	/// Name of the control group. May be relative (appended to the control group of the current process) or absolute (starting with "/").
	#[arg(short, long)]
	cgroup: String,

	/// The subcommand to run.
	#[arg(allow_hyphen_values(true))]
	cmd: Vec<String>,
}

fn main() {
	let args = Args::parse();
	common::os_check();
	let current_cgroup = CGroup::current();
	println!("{current_cgroup:?}");
	// TODO: Set the cgroup
	let status = Command::new(&args.cmd[0]).args(&args.cmd[1..]).status().unwrap();
	std::process::exit(status.code().unwrap_or(0))
}
