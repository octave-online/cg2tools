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
use clap::Args;
use clap::Parser;
use clap::Subcommand;

#[derive(Parser, Debug)]
#[command(version, about = "Manipulates settings for unified control groups (cgroups v2)")]
struct Cli {
	#[command(subcommand)]
	command: Command,
}

#[derive(Args, Debug)]
struct CreateCommand {
	/// Name of the control group. May be relative (appended to the control group of the current process) or absolute (starting with "/").
	#[arg()]
	cgroup: String,
}

#[derive(Args, Debug)]
struct ClassifyCommand {
	/// Name of the control group. May be relative (appended to the control group of the current process) or absolute (starting with "/").
	#[arg()]
	cgroup: String,

	/// Process IDs to reclassify.
	#[arg(value_delimiter = ',')]
	pids: Vec<u32>,

	/// Create the control group if it doesn't exist yet.
	#[arg(long, short)]
	auto: bool,
}

#[derive(Args, Debug)]
struct ControlCommand {
	/// Name of the control group. May be relative (appended to the control group of the current process) or absolute (starting with "/").
	#[arg()]
	cgroup: String,

	#[command(flatten)]
	control: ControlList,

	/// Create the control group if it doesn't exist yet.
	#[arg(long, short)]
	auto: bool,
}

#[derive(Args, Debug)]
#[group(multiple = false)]
struct ControlList {
	/// List of control to enable in the new control group.
	#[arg(value_delimiter = ',', value_parser = parse_controller_flag)]
	controllers: Vec<ControllerFlag>,

	/// Inherit all control from the specified control group, relative to the control group of the current process.
	#[arg(long, value_name = "CGROUP")]
	inherit: Option<String>,
}

impl ControlList {
	pub fn is_empty(&self) -> bool {
		self.controllers.is_empty() && self.inherit.is_none()
	}
}

#[derive(Debug, Clone)]
struct ControllerFlag {
	pub name: String,
	pub enable: bool,
}

fn parse_controller_flag(input: &str) -> Result<ControllerFlag, &'static str> {
	if let Some(name) = input.strip_prefix('+') {
		Ok(ControllerFlag {
			name: name.to_string(),
			enable: true,
		})
	} else {
		Err("controllers may only be enabled for now. Pass them with +, as in: +cpu +memory")
	}
}

#[derive(Args, Debug)]
struct RestrictCommand {
	/// Name of the control group. May be relative (appended to the control group of the current process) or absolute (starting with "/").
	#[arg()]
	cgroup: String,

	/// Restrictions to apply in file=value format, such as "cpu.weight=150". See <https://docs.kernel.org/admin-guide/cgroup-v2.html>
	#[arg(value_parser = parse_key_value)]
	restrictions: Vec<(String, String)>,

	/// Create the control group if it doesn't exist yet and enable the required controllers if they aren't enabled yet.
	#[arg(long, short)]
	auto: bool,
}

fn parse_key_value(input: &str) -> Result<(String, String), &'static str> {
	let (key, value) = input.split_once('=').ok_or("expected key=value")?;
	if !key.chars().all(|c| matches!(c, '_' | '.' | 'a'..='z')) {
		return Err("key contains invalid characters");
	}
	if !key.contains('.') {
		return Err("key must be of the form CONTROLLER.RESTRICTION");
	}
	Ok((key.to_string(), value.to_string()))
}

#[derive(Subcommand, Debug)]
enum Command {
	/// Creates a new control group
	Create(CreateCommand),
	/// Moves a running process to a different control group
	Classify(ClassifyCommand),
	/// Recursively lists or enables controllers in a control group
	Control(ControlCommand),
	/// Sets restrictions in a control group
	Restrict(RestrictCommand),
}

fn main() {
	let args = Cli::parse();
	internal::os_check(&args);
	let mut cgroup = CGroup::current();
	match args.command {
		Command::Create(cmd_args) => {
			cgroup.append(&cmd_args.cgroup);
			cgroup.create();
		}
		Command::Classify(cmd_args) => {
			cgroup.append(&cmd_args.cgroup);
			if cmd_args.auto {
				cgroup.create();
			}
			for pid in cmd_args.pids {
				cgroup.classify(pid);
			}
		}
		Command::Control(cmd_args) if cmd_args.control.is_empty() => {
			cgroup.append(&cmd_args.cgroup);
			if cmd_args.auto {
				cgroup.create();
			}
			let controllers = cgroup.controllers();
			println!("Controllers enabled in {cgroup}: {controllers:?}");
		}
		Command::Control(cmd_args) => {
			let mut anchor = None;
			let controllers: Vec<&str> = if let Some(inherit_cgroup_name) = cmd_args.control.inherit {
				let mut inherit_cgroup = cgroup.clone();
				inherit_cgroup.append(&inherit_cgroup_name);
				// Note: even with --auto, don't create the inherit cgroup
				let vec = anchor.insert(inherit_cgroup.controllers());
				vec.iter().map(|s| s.as_str()).collect()
			} else {
				cmd_args.control.controllers.iter().map(|c| c.name.as_str()).collect()
			};
			cgroup.append(&cmd_args.cgroup);
			if cmd_args.auto {
				cgroup.create();
			}
			cgroup.enable_controllers(controllers.as_slice());
		}
		Command::Restrict(cmd_args) => {
			cgroup.append(&cmd_args.cgroup);
			if cmd_args.auto {
				cgroup.create();
			}
			for (key, value) in cmd_args.restrictions.iter() {
				if cmd_args.auto {
					cgroup.enable_controller_for_restriction(key);
				}
				cgroup.set_restriction(key, value);
			}
		}
	}
}
