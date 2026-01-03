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
#[command(version, about = "Sets restrictions in a control group")]
struct Args {
	/// Name of the control group. May be relative (appended to the control group of the current process) or absolute (starting with "/").
	#[arg(short = 'g', long)]
	cgroup: String,

	/// Restrictions to apply in file=value format, such as "cpu.weight=150". See <https://docs.kernel.org/admin-guide/cgroup-v2.html>
	#[arg(short = 'r', long, value_parser = parse_key_value)]
	restrictions: Vec<(String, String)>,
}

fn parse_key_value(input: &str) -> Result<(String, String), &'static str> {
	let (key, value) = input.split_once('=').ok_or("expected key=value")?;
	if !key.chars().all(|c| matches!(c, '_' | '.' | 'a'..='z')) {
		return Err("key contains invalid characters");
	}
	Ok((key.to_string(), value.to_string()))
}

fn main() {
	let args = Args::parse();
	internal::os_check();
	let mut cgroup = CGroup::current();
	cgroup.append(&args.cgroup);
	for (key, value) in args.restrictions.iter() {
		cgroup.set_restriction(key, value);
	}
}
