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
use std::ffi::OsString;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(version, about = "Runs a program with a specific control group")]
struct Cli {
	/// Name of the control group. May be relative (appended to the control group of the current process) or absolute (starting with "/").
	#[arg()]
	cgroup: String,

	/// The subcommand to run.
	#[arg()]
	cmd: OsString,

	/// Arguments to the subcommand.
	#[arg(allow_hyphen_values(true))]
	args: Vec<OsString>,
}

fn main() {
	let args = Cli::parse();
	internal::os_check(&args);
	let mut cgroup = CGroup::current();
	if cgroup.append(&args.cgroup) {
		cgroup.classify_current();
	}
	let status = Command::new(&args.cmd).args(&args.args).status().unwrap();
	std::process::exit(status.code().unwrap_or(0))
}

#[test]
fn test_cli() {
	fn cli(input: &str) -> Result<Cli, String> {
		Cli::try_parse_from(shlex::split(input).unwrap()).map_err(|e| format!("{e}"))
	}
	insta::assert_debug_snapshot!(cli("cg2exec"));
	insta::assert_debug_snapshot!(cli("cg2exec grp"));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd"));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd extra"));
	insta::assert_debug_snapshot!(cli("cg2exec --flag grp cmd"));
	insta::assert_debug_snapshot!(cli("cg2exec grp --flag cmd"));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd --flag"));
}
