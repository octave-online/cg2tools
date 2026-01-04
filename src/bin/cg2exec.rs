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

use argh::FromArgs;
use cg2tools::internal;
use cg2tools::CGroup;
use std::process::Command;

/// Runs a program with a specific control group
#[derive(FromArgs, Debug)]
struct Cli {
	/// name of the control group. May be relative (appended to the control group of the current process) or absolute (starting with "/").
	#[argh(positional)]
	cgroup: String,

	/// the subcommand to run
	#[argh(positional)]
	cmd: String,

	/// arguments to the subcommand
	#[argh(positional, greedy)]
	args: Vec<String>,
}

fn main() {
	let args = argh::from_env::<Cli>();
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
		let tokens = shlex::split(input).unwrap();
		let tokens = tokens.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
		Cli::from_args(&[tokens[0]], &tokens[1..]).map_err(|e| format!("{e:?}"))
	}
	insta::assert_debug_snapshot!(cli("cg2exec"));
	insta::assert_debug_snapshot!(cli("cg2exec grp"));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd"));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd extra"));
	insta::assert_debug_snapshot!(cli("cg2exec --flag grp cmd"));
	insta::assert_debug_snapshot!(cli("cg2exec grp --flag cmd"));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd --flag"));
}
