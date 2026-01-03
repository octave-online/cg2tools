cg2tools
========

This package contains lightweight CLI tools for manipulating Unified Control Groups (also known as cgroups v2) in the Linux kernel via the cgroupfs.

The tools are primarily designed for services with `Delegate=yes` in their [systemd configuration](https://systemd.io/CGROUP_DELEGATION/).

***Why use cg2tools instead of writing directly to cgroupfs?*** Relying on bash or python scripting is error-prone, and cgroupfs has a steep learning curve. cg2tools is built for the community of Linux developers who wish to use control groups with ergonomic CLI tools, such as the ones we had with cgroups v1. `cg2exec` in particular is a command that has no good equivalent in cgroupfs.

## Tools

There are currently two tools:

- `cg2exec` for running subcommands in specific cgroups.
- `cg2util` for configuring cgroups and classifying existing processes.

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

Equivalent cgroupfs command:

```bash
$ mkdir /sys/fs/cgroup/path/to/my.service/my_subgroup
```

**Example 2:** Reclassify the current process into a subgroup, creating the subgroup if it doesn't exist.

```bash
$ cg2util classify --auto my_subgroup $$
```

Equivalent cgroupfs command (does NOT automatically create the group):

```bash
$ echo $$ > /sys/fs/cgroup/path/to/my.service/my_subgroup/cgroup.procs
```

**Example 3:** Allow the group /custom/cpulimit to manipulate CPU restrictions.

```bash
$ cg2util control /custom/cpulimit +cpu
```

Equivalent cgroupfs command:

```bash
$ echo +cpu > /sys/fs/cgroup/Custom/cgroup.subtree_control
```

**Example 4:** Restrict the group /custom/cpulimit to 90% of CPU, enforced in periods lasting 100ms. Create the group if it doesn't exist, and allow it to set that restriction.

```bash
$ cg2util restrict --auto /custom/cpulimit cpu.max="90000 100000"
```

Equivalent cgroupfs command (does NOT automatically create the group or enable the controller):

```bash
$ echo "90000 100000" > /sys/fs/cgroup/Custom/cpulimit/cpu.max
```

## Installation

Install from the Cargo package manager.

## Example: Integration with systemd service

This example will use an unprivileged user to demonstrate that root permissions are not required by the service:

```bash
$ useradd -m -u 1500 cg2tools_user;
```

Create the service config at `/etc/systemd/system/cg2tools_demo.service`

```ini
[Service]
ExecStart=/usr/local/share/cg2tools_demo.sh
User=cg2tools_user
Group=cg2tools_user
Restart=no
Delegate=yes

[Install]
WantedBy=multi-user.target
```

Create the service as a bash script at `/usr/local/share/cg2tools_demo.sh` (although with cg2tools, a bash script wrapper is not compulsory)

```bash
#!/bin/bash

# Move the main process into a new cgroup called main.
# We need to do this because we can't reconfigure cgroups
# that have processes running in them.
cg2util classify --auto main $$

# Create cgroup subproc and restrict it to 50ms of CPU time every 100ms
cg2util restrict --auto ../subproc cpu.max=50000

# Create tier1 and tier2 with different CPU weights
cg2util restrict --auto ../subproc/tier1 cpu.weight=150
cg2util restrict --auto ../subproc/tier2 cpu.weight=50

# Now let's spawn some subcommands.
cg2exec ../subproc/tier1 stress -c 1 &
cg2exec ../subproc/tier2 stress -c 1 &

# Wait 2 minutes and then shut down the service
sleep 120
```

If the `stress` command is unavailable, install it from your favorite package manager.

Fire up the new service:

```bash
$ sudo systemctl daemon-reload
$ sudo systemctl start cg2tools_demo
```

Watch the system monitor to see the control group limits in action.

### Example setup without --auto

The equivalent of the above script without using `--auto`:

```bash
#!/bin/bash

# Create some empty cgroups
cg2util create main
cg2util create subproc
cg2util create subproc/tier1
cg2util create subproc/tier2

# Move the main process into the main cgroup.
# We need to do this because we can't reconfigure cgroups
# that have processes running in them.
cg2util classify main $$

# Enable CPU limits on the other cgroups
cg2util control ../subproc +cpu
cg2util control ../subproc/tier1 +cpu
cg2util control ../subproc/tier2 +cpu

# Restrict subproc to 50ms of CPU time every 100ms
cg2util restrict ../subproc cpu.max=50000

# Give tier1 more CPU weight than tier2
cg2util restrict ../subproc/tier1 cpu.weight=150
cg2util restrict ../subproc/tier2 cpu.weight=50

# Now let's spawn some subcommands.
cg2exec ../subproc/tier1 stress -c 1 &
cg2exec ../subproc/tier2 stress -c 1 &

# Wait 2 minutes and then shut down the service
sleep 120
```
```

## Copyright and License

See [NOTICE](NOTICE) and [LICENSE](LICENSE).
