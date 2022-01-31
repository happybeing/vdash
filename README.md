# SAFE Network Node Dashboard

`vdash` is a terminal based dashboard for monitoring SAFE Network Nodes. It is written in
Rust, using [tui-rs](https://github.com/fdehau/tui-rs) to create the terminal UI
and [linemux](https://github.com/jmagnuson/linemux) to monitor node logfiles on
the local machine.

**Status:** working on Windows, MacOS and Linux with local and public test networks.

`vdash` is already capable of monitoring multiple logfiles on the local machine
so it wouldn't be hard to monitor multiple remote nodes by having a script pull
logfiles from remote nodes to local copies monitored by `vdash`, but this is not
on the roadmap. There may be existing tools that could do this so if anyone
wants to look into that it would be great as I'm only making minor changes at
the moment.

Here's an early `vdash` (v0.2.0) working with a local testnet node:
<img src="./screenshots/vdash-v.0.2.4.gif" alt="screenshot of vdash v0.2.0">

## Features
`vdash` will load historic metrics from one or more Safe node
logfiles and display these with live updates in the terminal (see above).

You can cycle through different Safe nodes using left/right arrow
keys, and zoom the timeline scale in/out using 'i' and 'o' (or '+' and '-').

Press 'q' to quit.

Feature requests and discussion are currently summarised in the opening post of
the Safe Network forum topic: [Node Dashboard ideas
please!](https://safenetforum.org/t/node-dashboard-ideas-please/32572?u=happybeing).

For more details and progress see [Roadmap](#roadmap) (below).

## Operating Systems
- **Linux:** works on Linux (tested on Ubuntu).
- **Windows:** works on Windows 10.
- **MacOS:** works on MacOS.

## Install from crates.io

1 Install **Rust** via https://doc.rust-lang.org/cargo/getting-started/installation.html

2a. **Linux (Ubuntu)**

    sudo apt-get install build-essential

2b. **Linux/MacOS** install **vdash:**

    cargo install vdash
    vdash --help

2c. **Windows** install **vdash-crossterm:**

To install on Windows you must build manually and use the 'nightly' compiler
until the 'itarget' feature becomes part of 'stable', so install Rust nightly
using `rustup`:

    rustup toolchain install nightly

To build `vdash-crossterm` on Windows, clone vdash, build with `+nightly` and use the binary it creates under `./taget/release`:

    git clone https://github.com/happybeing/vdash
    cd vdash
    cargo +nightly build -Z features=itarget --bin vdash-crossterm --release --no-default-features

    ./target/release/vdash-crossterm --help

## Using vdash - SAFE Network Node Dashboard
`vdash` provides a terminal based graphical dashboard of SAFE Network Node activity on the local machine. It parses input from one or more node logfiles to gather live node metrics which are displayed using terminal graphics.

**Status:** useful work-in-progress, help welcome!

## Get SAFE Network pre-requisites
1. **Get Rust:** see: https://doc.rust-lang.org/cargo/getting-started/installation.html

2. **Get the SAFE CLI:** either download using an install script or build the SAFE CLI locally. Instructions for both options are [here](https://github.com/maidsafe/sn_api/tree/master/sn_cli#safe-network-cli).

3. **Get the SAFE Node:** when you have the SAFE CLI working you can install the node software with the command ` safe node install` (details [here](https://github.com/maidsafe/sn_api/tree/master/sn_cli#node-install)).

You are now ready to install `vdash` and can test it by running a local test network.

## Usage

In the terminal type the command and the paths of one or more node logfiles you want to monitor. For example:

    vdash ~/.safe/node/local-node/sn_node.log

When the dashboard is active, pressing 's' or 'd' switches between summary and detail views.
For more information:

    vdash --help

### Node Setup
**IMPORTANT:** You must ensure the node logfile includes the telemetry information used by vdash by setting the logging level to 'trace' when you start your node (as in the example below). You control the node logging level by setting the environment variable `RUST_LOG`.

```sh
safe node killall
rm -f ~/.safe/node/local-test-network/*/sn_node.log*
RUST_LOG=safe_network=trace safe node join
```
You can then run `vdash`, typically in a different terminal:
```sh
vdash ~/.safe/node/local-node/sn_node.log
```
Or
Note:
- `safe node killall` makes sure no existing nodes are still running, and
  deleting existing logfiles prevents you picking up statistics from previous
  activity. If you leave the logfile in place then `vdash` will waste time
  processing that, although you can skip that process using a command line
  option.
- setting RUST_LOG ensures the logfiles contain the data which vdash needs, and
  excludes some that gets in the way.
- On Windows to set RUST_LOG environment variable:

	Using Windows Command Line:
	```
	set RUST_LOG="safe_network=trace"
	safe node join
	```

	Using Windows PowerShell:
	```
	$env:RUST_LOG="safe_network=trace"
	safe node join
	```

Note: the examples use `safe node join` to start a node, but you will need to check you are using the correct parameters for your purpose. For example you might want to skip port forwarding etc.
### Using vdash With a Local Test Network
1. **Start a local test network:** follow the instructions to [Run a local network](https://github.com/maidsafe/sn_api/tree/master/sn_cli#run-a-local-network), but I suggest using the `-t` option to create an account and authorise the CLI with it altogether. As here:
    ```
    safe node killall
    rm -f ~/.safe/node/baby-fleming-nodes/*/sn_node.log
    RUST_LOG=safe_network=trace safe node run-baby-fleming -t
    ```

	Windows: see "Note" immediately above for how to set RUST_LOG on Windows.

2. **Run vdash:** in a different terminal window (so you can continue to use the safe-cli in the first terminal), start `vdash` with:
    ```
    vdash ~/.safe/node/baby-fleming-nodes/*/sn_node.log
    ```
    Or with a live network:
    ```
    vdash ~/.safe/node/local-node/sn_node.log
    ```
3. **Upload files using SAFE CLI:** in the SAFE CLI window you can perform operations on the local test network that will affect the node and the effects will be shown in `vdash`. For example, to [use the SAFE CLI to upload files](https://github.com/maidsafe/sn_api/tree/master/sn_cli#files):
    ```
    safe files put ./<some-directory>/ --recursive
    ```

If you want to try `vdash` with a live network, check to see if one is running at the SAFE Network community forum: https://safenetforum.org

## Build

See [Get SAFE Network Pre-requisites](#get-safe-network-pre-requisites).

### Get code
```
git clone https://github.com/happybeing/vdash
cd vdash
```

### Build - Linux / MacOS
Build `vdash` with the termion backend (see [tui-rs](https://github.com/fdehau/tui-rs)).
Note: MacOS is untested but may 'just work'.
```
cargo build --features="termion" --features="vdash" --release
```
If built for target 'musl' `vdash` uses considerably less memory:

```sh
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```
Comparing memory use (using `htop` on Linux):
```sh
VIRT   RES  SHR
803M  9372 4716 x13 threads (release)
32768 6848 2440 x13 threads (release/musl)
```
#### Build - Windows 10
Builds `vdash` the crossterm backend (see [tui-rs](https://github.com/fdehau/tui-rs)), with the intention to support Windows.

NOT working on Windows yet, this is being worked on at the moment. Help with testing appreciated.
```
cargo build --bin vdash-crossterm --features="crossterm" --features="vdash" --release
```


# Roadmap
Where `vdash` is headed:
- [x] implement ability to parse logfiles
  - [x] add --debug-parser to show results in second logfile
  - [x] implement parsing log file for simple metrics and timeline
  - [x] keep the debug UI available (selected with 'D' when using --debug-parse)
- [x] change events to use tokio mpsc (unbounded) channel
- [x] does tokio mpsc fix loss of updates from linemux (see linemux [issue #17](https://github.com/jmagnuson/linemux/issues/17))
- [ ] implement node dashboard
  - [x] node status summary page (single node)
  - [x] debug window (--debug-window)
  - [x] add basic node stats (age/PUTs/GETs)
  - [x] scroll node logfile (arrow keys)
  - [x] multiple nodes (navigate with tab and arrow keys)
  - [ ] add a timeline
    - [x] simple timeline with PUTS and GETS
    - [x] implement multiple timeline durations (hour, minute etc)
    - [x] add status/timeline for ERRORS
    - [x] anchor 'now' to right border
    - [ ] mod sparkline widget to have a minimum Y scale (e.g. 10 units)
  - [ ] reduce lag in processing logfile changes
    - [x] implement simple rate limit on redraws
    - [x] implement update/redraw tick (for timeline and stats)
    - [x] fix load from logfile to timeline (currently all ends up in last bucket)
    - [x] change timeline scaling to use +/- an i/o keys rather than s, m, d etc
    - [ ] optimise redraw rate limit
    - [ ] make a CLI option for redraw rate limit
  - [ ] track sn_node [issue #1126](https://github.com/maidsafe/sn_node/issues/1126) (maintain Get/Put response in)
  - [x] implement storage 'meter'
    - [x] code to get node storage used
    - [x] code to get free space on same device
    - [x] implement storage used/free 'progress' bar
  - [ ] implement bandwidth 'meter'
    - [ ] code to get node bandwidth
    - [ ] code to get total bandwidth
    - [ ] implement bandwidth node/total/max in last day 'progress' bar
- [ ] Implement DashOverview: all nodes on one page (rename from DashSummary)
- [ ] trim NodeMetrics timeline
- [ ] logtail-dash [Issue #1](https://github.com/happybeing/logfile-dash/issues/1): Implement popup help on ?, h, H
- [x] FIXED by upate to tui-rs v0.11.0 [Issue #382](https://github.com/fdehau/tui-rs/issues/382): Window titles corrupted when using --debug-window
- [ ] Implement --features="vdash" / --features="logtail" to select app and UI


## LICENSE

Everything is GPL3.0 unless otherwise stated. Any contributions are accepted on the condition they conform to this license.

See also [./LICENSE](./LICENSE)