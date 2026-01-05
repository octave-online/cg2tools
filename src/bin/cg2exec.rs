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
use clap_lex::RawArgs;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::io::Write;
use std::process::Command;

#[derive(Debug)]
struct Cli {
	/// Name of the control group. May be relative (appended to the control group of the current process) or absolute (starting with "/").
	cgroup: String,

	/// The subcommand to run.
	cmd: OsString,

	/// Arguments to the subcommand.
	args: Vec<OsString>,
}

enum CliRequest {
	Cli(Cli),
	Help { bin_name: OsString },
	Version,
}

enum CliError {
	Unexpected { arg: OsString, bin_name: OsString },
	InvalidCgroup { arg: OsString, bin_name: OsString },
	MissingCgroup { bin_name: OsString },
	MissingCommand { bin_name: OsString },
}

impl CliError {
	fn bin_name(&self) -> &OsStr {
		match self {
			Self::Unexpected { bin_name, .. } => &*bin_name,
			Self::InvalidCgroup { bin_name, .. } => &*bin_name,
			Self::MissingCgroup { bin_name, .. } => &*bin_name,
			Self::MissingCommand { bin_name, .. } => &*bin_name,
		}
	}
}

impl fmt::Display for CliError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::Unexpected { arg, .. } => {
				write!(f, "Unexpected flag or argument: {arg:?}")
			}
			Self::InvalidCgroup { arg, .. } => {
				write!(f, "Invalid control group name: {arg:?}")
			}
			Self::MissingCgroup { .. } => write!(f, "Missing control group"),
			Self::MissingCommand { .. } => write!(f, "Missing subcommand"),
		}
	}
}

impl TryFrom<RawArgs> for CliRequest {
	type Error = CliError;
	fn try_from(raw: RawArgs) -> Result<Self, CliError> {
		let mut cursor = raw.cursor();
		let bin_name = raw.next(&mut cursor).unwrap().to_value_os().to_os_string();
		let mut escape = false;
		let cgroup = match raw.next(&mut cursor) {
			Some(arg) => match (&arg, arg.to_long(), arg.to_value()) {
				(_, Some((Ok("help"), _)), _) => {
					return Ok(CliRequest::Help { bin_name });
				}
				(_, Some((Ok("version"), _)), _) => {
					return Ok(CliRequest::Version);
				}
				(arg, _, _) if arg.is_escape() => {
					escape = true;
					match raw.next(&mut cursor) {
						Some(arg) => match arg.to_value() {
							Ok(s) => s.to_string(),
							Err(s) => {
								return Err(CliError::InvalidCgroup {
									arg: s.to_os_string(),
									bin_name,
								})
							}
						},
						None => return Err(CliError::MissingCgroup { bin_name }),
					}
				}
				(arg, _, _) if arg.is_stdio() || arg.is_long() || arg.is_short() => {
					return Err(CliError::Unexpected {
						arg: arg.to_value_os().to_os_string(),
						bin_name,
					});
				}
				(_, _, Ok(s)) => s.to_string(),
				(_, _, Err(s)) => {
					return Err(CliError::InvalidCgroup {
						arg: s.to_os_string(),
						bin_name,
					});
				}
			},
			None => return Err(CliError::MissingCgroup { bin_name }),
		};
		let cmd = match raw.next(&mut cursor) {
			Some(arg) if !escape && (arg.is_escape() || arg.is_stdio() || arg.is_long() || arg.is_short()) => {
				return Err(CliError::Unexpected {
					arg: arg.to_value_os().to_os_string(),
					bin_name,
				});
			}
			Some(arg) => arg.to_value_os().to_os_string(),
			None => return Err(CliError::MissingCommand { bin_name }),
		};
		let args = raw.remaining(&mut cursor).map(|s| s.to_os_string()).collect();
		Ok(CliRequest::Cli(Cli { cgroup, cmd, args }))
	}
}

fn print_description(mut sink: impl Write) -> Result<(), io::Error> {
	writeln!(sink, "Runs a program with a specific control group")
}

fn print_usage(bin_name: &OsStr, mut sink: impl Write) -> Result<(), io::Error> {
	writeln!(sink, "Usage: {} <CGROUP> <CMD> [ARGS]...", bin_name.to_string_lossy())
}

impl Cli {
	pub fn try_from_env(sink: impl Write) -> Result<Cli, i32> {
		Self::try_new_raw(RawArgs::from_args(), sink)
	}

	#[cfg(test)]
	pub fn try_from_tokens(tokens: impl Iterator<Item = impl Into<OsString>>, sink: impl Write) -> Result<Cli, i32> {
		Self::try_new_raw(RawArgs::new(tokens), sink)
	}

	fn try_new_raw(raw: RawArgs, mut sink: impl Write) -> Result<Cli, i32> {
		match CliRequest::try_from(raw) {
			Ok(CliRequest::Cli(cli)) => Ok(cli),
			Ok(CliRequest::Help { bin_name }) => {
				print_description(&mut sink).unwrap();
				print_usage(&*bin_name, &mut sink).unwrap();
				Err(0)
			}
			Ok(CliRequest::Version) => {
				writeln!(&mut sink, "cg2tools {}", clap::crate_version!()).unwrap();
				Err(0)
			}
			Err(e) => {
				writeln!(&mut sink, "Error: {e}").unwrap();
				print_usage(e.bin_name(), &mut sink).unwrap();
				Err(1)
			}
		}
	}
}

fn main() {
	let args = match Cli::try_from_env(std::io::stderr()) {
		Ok(args) => args,
		Err(code) => std::process::exit(code),
	};
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
		let mut buf = Vec::<u8>::new();
		match Cli::try_from_tokens(tokens.iter(), &mut buf) {
			Ok(args) => Ok(args),
			Err(_code) => Err(String::from_utf8(buf).unwrap()),
		}
	}
	insta::assert_debug_snapshot!(cli("cg2exec"));
	insta::assert_debug_snapshot!(cli("cg2exec grp"));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd"));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd extra"));
	insta::assert_debug_snapshot!(cli("cg2exec --flag grp cmd"));
	insta::assert_debug_snapshot!(cli("cg2exec grp --flag cmd"));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd --flag"));
	insta::assert_debug_snapshot!(cli("cg2exec -- grp cmd extra"));
	insta::assert_debug_snapshot!(cli("cg2exec grp -- cmd extra"));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd -- extra"));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd extra --"));
	insta::assert_debug_snapshot!(cli("cg2exec -- -grp -cmd -extra"));
}
