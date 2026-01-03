cg2tools
========

This package contains lightweight CLI tools for manipulating Unified Control Groups (also known as cgroups v2) in the Linux kernel via the cgroupfs.

The tools are primarily designed for services with `Delegate=yes` in their [systemd configuration](https://systemd.io/CGROUP_DELEGATION/).

## Tools

There are currently two tools:

- `cg2exec` for running subcommands in specific cgroups.
- `cg2util` for configuring cgroups.

Control groups can be specified as either relative or absolute paths.

### cg2exec

Use this tool to run a subcommand in a specific control group.

**Example 1:** Run a command in the cgroup `subgroup`, a child of the current process's cgroup. Assuming an appropriate cgroup setup, this command should work without needing extra permissions.

```bash
$ cg2exec subgroup echo "Running in a subgroup of the execution environment"
```

**Example 2:** Run a command in the cgroup `/custom`, a cgroup that is a direct child of the root cgroup. This command probably requires root permissions.

```bash
$ cg2exec /custom echo "Running in the subgroup /custom"
```

### cg2util

Use this tool to create and configure control groups.

**Example 1:** Create a new subgroup `my_subgroup` as a child of the current process's cgroup.

```bash
$ cg2util create my_subgroup
```

**Example 2:** Allow the subgroup to manipulate CPU restrictions.

```bash
$ cg2util control my_subgroup +cpu
```

**Example 3:** Restrict the subgroup to 80% of CPU, enforced in periods lasting 100ms.

```bash
$ cg2util restrict my_subgroup cpu.max="90000 100000"
```

## Installation

Install from the Cargo package manager.

## Example: Integration with systemd service

To be added soon.

## Copyright and License

See [NOTICE](NOTICE) and [LICENSE](LICENSE).
