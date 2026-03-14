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
struct CliExecCommand<'a> {
	/// Name of the control group. May be relative (appended to the control group of the current process) or absolute (starting with "/").
	cgroup: &'a OsStr,

	/// The subcommand to run.
	cmd: &'a OsStr,

	/// Arguments to the subcommand.
	args: Vec<&'a OsStr>,
}

struct CliHelpCommand<'a> {
	bin_name: &'a OsStr,
}

enum CliCommand<'a> {
	Exec(CliExecCommand<'a>),
	Help(CliHelpCommand<'a>),
	Version,
}

struct CliError<'a> {
	bin_name: &'a OsStr,
	kind: CliErrorKind<'a>,
}

enum CliErrorKind<'a> {
	Unexpected { arg: &'a OsStr },
	InvalidCgroup { arg: &'a OsStr },
	MissingCgroup,
	MissingCommand,
}

impl fmt::Display for CliError<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
		match self.kind {
			CliErrorKind::Unexpected { arg } => {
				write!(f, "Unexpected flag or argument: {arg:?}")
			}
			CliErrorKind::InvalidCgroup { arg } => {
				write!(f, "Invalid control group name: {arg:?}")
			}
			CliErrorKind::MissingCgroup => write!(f, "Missing control group"),
			CliErrorKind::MissingCommand => write!(f, "Missing subcommand"),
		}
	}
}

impl<'a> TryFrom<&'a RawArgs> for CliCommand<'a> {
	type Error = CliError<'a>;
	fn try_from(raw: &'a RawArgs) -> Result<CliCommand<'a>, CliError<'a>> {
		let mut cursor = raw.cursor();
		let bin_name = raw.next(&mut cursor).unwrap().to_value_os();
		let mut escape = false;
		let cgroup = match raw.next(&mut cursor) {
			Some(arg) => match (arg.to_long(), &arg) {
				(Some((Ok("help"), _)), _) => {
					return Ok(CliCommand::Help(CliHelpCommand { bin_name }));
				}
				(Some((Ok("version"), _)), _) => {
					return Ok(CliCommand::Version);
				}
				(_, arg) if arg.is_escape() => {
					escape = true;
					match raw.next(&mut cursor) {
						Some(arg) => arg.to_value_os(),
						None => return Err(CliError { bin_name, kind: CliErrorKind::MissingCgroup }),
					}
				}
				(_, arg) if arg.is_stdio() || arg.is_long() || arg.is_short() => {
					return Err(CliError {
						bin_name,
						kind: CliErrorKind::Unexpected {
							arg: arg.to_value_os(),
						}
					});
				}
				(_, arg) => arg.to_value_os(),
			},
			None => return Err(CliError { bin_name, kind: CliErrorKind::MissingCgroup }),
		};
		let cmd = match raw.next(&mut cursor) {
			Some(arg) if !escape && (arg.is_escape() || arg.is_stdio() || arg.is_long() || arg.is_short()) => {
				return Err(CliError {
					bin_name,
					kind: CliErrorKind::Unexpected {
						arg: arg.to_value_os(),
					}
				});
			}
			Some(arg) => arg.to_value_os(),
			None => return Err(CliError { bin_name, kind: CliErrorKind::MissingCommand }),
		};
		let args = raw.remaining(&mut cursor).collect();
		Ok(CliCommand::Exec(CliExecCommand { cgroup, cmd, args }))
	}
}

fn print_description(mut sink: impl Write) -> Result<(), io::Error> {
	writeln!(sink, "Runs a program with a specific control group")
}

fn print_usage(bin_name: &OsStr, mut sink: impl Write) -> Result<(), io::Error> {
	writeln!(sink, "Usage: {} <CGROUP> <CMD> [ARGS]...", bin_name.to_string_lossy())
}

impl<'a> CliExecCommand<'a> {
	pub fn try_from_raw(raw: &'a RawArgs, mut sink: impl Write) -> Result<CliExecCommand, i32> {
		match CliCommand::try_from(raw) {
			Ok(CliCommand::Exec(cli)) => Ok(cli),
			Ok(CliCommand::Help(CliHelpCommand { bin_name })) => {
				print_description(&mut sink).unwrap();
				print_usage(&*bin_name, &mut sink).unwrap();
				Err(0)
			}
			Ok(CliCommand::Version) => {
				writeln!(&mut sink, "cg2tools {}", clap::crate_version!()).unwrap();
				Err(0)
			}
			Err(e) => {
				writeln!(&mut sink, "Error: {e}").unwrap();
				print_usage(e.bin_name, &mut sink).unwrap();
				Err(1)
			}
		}
	}
}

fn main() {
	let raw_args = RawArgs::from_args();
	let args = match CliExecCommand::try_from_raw(&raw_args, std::io::stderr()) {
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
	let mut a: Option<RawArgs> = None;
	fn cli<'s>(input: &str, anchor: &'s mut Option<RawArgs>) -> Result<CliExecCommand<'s>, String> {
		let tokens = shlex::split(input).unwrap();
		let mut buf = Vec::<u8>::new();
		let raw_args = anchor.insert(RawArgs::new(tokens));
		match CliExecCommand::try_from_raw(raw_args, &mut buf) {
			Ok(args) => Ok(args),
			Err(_code) => Err(String::from_utf8(buf).unwrap()),
		}
	}
	insta::assert_debug_snapshot!(cli("cg2exec", &mut a));
	insta::assert_debug_snapshot!(cli("cg2exec grp", &mut a));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd", &mut a));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd extra", &mut a));
	insta::assert_debug_snapshot!(cli("cg2exec --flag grp cmd", &mut a));
	insta::assert_debug_snapshot!(cli("cg2exec grp --flag cmd", &mut a));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd --flag", &mut a));
	insta::assert_debug_snapshot!(cli("cg2exec -- grp cmd extra", &mut a));
	insta::assert_debug_snapshot!(cli("cg2exec grp -- cmd extra", &mut a));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd -- extra", &mut a));
	insta::assert_debug_snapshot!(cli("cg2exec grp cmd extra --", &mut a));
	insta::assert_debug_snapshot!(cli("cg2exec -- -grp -cmd -extra", &mut a));
}
