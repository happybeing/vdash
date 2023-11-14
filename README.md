# Safe Network node Dashboard

`vdash` is a terminal based dashboard for monitoring Safe Network nodes. It is written in
Rust. The terminal GUI is implemented using [ratatui](https://github.com/ratatui-org/ratatui) and it monitors one or more node logfiles using [linemux](https://github.com/jmagnuson/linemux).

**Status:** working on Windows, MacOS and Linux with public test networks.

`vdash` is already capable of monitoring multiple logfiles on the local machine, showing multiple metrics for each node including number of PUTS (chunks stored), current price being charged for storage, and node earnings. Many metrics appear both as numeric values and can be viewed in real-time graphical charts over time.

Using `rsyslog` it should be possible to monitor logfiles for the local machine and from multiple remote machines too, though I have not tried this myself yet.

Here's an early `vdash` (v0.2.0) working with a local testnet node:
<img src="./screenshots/vdash-v.0.2.4.gif" alt="screenshot of vdash v0.2.0">

## Features
`vdash` will load historic metrics from one or more Safe node
logfiles and display these with live updates in the terminal (see above).

**'<-' and '->':** When monitoring multiple nodes you can cycle through
them using the left/right arrow keys.

**'i' and 'o':** Zoom the timeline scale in/out using 'i' and 'o' (or '+' and '-').

**'t' and 'T':** Three timelines are visible at any one time but you can cycle
through all timelines to bring them all into view by pressing 't' (forward) and 'T'
(backward).

**'m' or 'M':** The Storage Cost timeline displays minimum, mean and maximum
values in each time-slot. To cycle through the min, mean and max displays
press 'm' or 'M'.

**'q':** Press 'q' to quit.

Feature requests and discussion are currently summarised in the opening post of
the Safe Network forum topic: [node Dashboard ideas
please!](https://safenetforum.org/t/node-dashboard-ideas-please/32572?u=happybeing).

For more details and progress see [Roadmap](#roadmap) (below).

## Operating Systems
- **Linux:** works on Linux (tested on Ubuntu).
- **Windows:** works on Windows 10. Not tested recently.
- **MacOS:** works on MacOS. Not tested recently.

## Install using Linux package manager

`vdash` has been packaged for debian thanks to the generous efforts of Jonas Smedegaard. From late 2023 it will begin to be available in many downstream Linux distributions, but due to the pace of updates the packaged version is likely to be behind the version published at crates.io which is always up to date.

You can check the status of package `safe-vdash` in your distribution and choose whether to install from there or get the most recent version as explained below.

## Install from crates.io

1 Install **Rust** via https://doc.rust-lang.org/cargo/getting-started/installation.html

2a. **Linux (Ubuntu)**

    sudo apt-get install build-essential

2b. **Linux/MacOS** install **vdash:**

    cargo install vdash
    vdash --help

2c. **Windows** install **vdash:**

Windows has not been tested recently so you may like to try using `cargo insall vdash` first to see if that now works. If not, you will need to build using Rust Nightly.

Until the 'itarget' feature becomes part of 'stable', build manually and use the Rust Nightly compiler first install Rust Nightly
using `rustup`:

    rustup toolchain install nightly

To build `vdash` on Windows, clone vdash, build with `+nightly` and use the binary it creates under `./taget/release`:

    git clone https://github.com/happybeing/vdash
    cd vdash
    cargo +nightly build -Z features=itarget --bin vdash --release --no-default-features

    ./target/release/vdash --help

## Using vdash - a Safe Network node Dashboard
`vdash` provides a terminal based graphical dashboard of Safe Network node activity on the local machine. It parses input from one or more node logfiles to gather live node metrics which are displayed using terminal graphics.


## Get Safe Network pre-requisites
1. **Get Rust:** see: https://doc.rust-lang.org/cargo/getting-started/installation.html

2. **Get the Safe CLI:** either download using an install script or build the Safe CLI locally. Instructions for both options are [here](https://github.com/maidsafe/sn_api/tree/master/sn_cli#safe-network-cli).

3. **Get the Safe node:** when you have the Safe CLI working you can install the node software with the command ` safe node install` (details [here](https://github.com/maidsafe/sn_api/tree/master/sn_cli#node-install)).

You are now ready to install `vdash` and can test it by running a local test network.

## Usage

For help:

    vdash --help

Typically you can just pass the paths of one or more node logfiles you want to monitor. For example, to run `vdash` first start your Safe Network node(s) with one or more `safenode` commands. Then, assuming their logfiles are in the standard location start `vdash` with:

**Linux:**

    vdash ~/.local/share/safe/node/*/logs/safenode.log

**Mac:**

    vdash "/Users/<username>/Library/Application Support/safe/node/*/logs/safenode.log"

**Windows:**

    vdash C:\Users\<username>\AppData\Roaming\safe\node\*\logs\safenode.log

Keyboard commands for `vdash` are summarised in the introduction above.

### vdash and 'glob' paths

`vdash` accepts one or more file paths, but you can also specify one or more 'glob' paths which can scan a directory tree for matching files. This enables you to pick up new nodes added after `vdash` starts, either using the 'r' (re-scan) keyboard command, or automatically by giving a re-scanning period using the `--glob-scan` option on the command line.

`vdash` scans all 'glob' paths provided on start-up and again whenever you press 'r'.

Note that unlike a file path you must use quotation marks around a 'glob' path to prevent the shell from trying to expand it. In the examples you will need to replace `<USER>` with the appropriate home directory name for your account.

Examples for Linux:

    vdash --glob-path "/home/<USER>/.local/share/safe/node/*/logs/safenode.log"

    vdash -g "$HOME/.local/share/safe/node/**/safenode.log" -g "./remote-node-logs/*/logs/safenode.log" --glob-scan 5

Using double rather than single quotes enables you to use '$HOME' in the path instead of giving the home directory explicitly.

### Safe Node Setup

**IMPORTANT:** You must ensure the node logfile includes the telemetry information used by vdash by setting the logging level to 'trace' when you start your node (as in the example below). You control the node logging level by setting the environment variable `SN_LOG`.

```sh
killall safenode
rm -rf ~/.local/share/safe/node
SN_LOG=all safenode
```
To start a node using `safenode` you should check you are using the correct parameters for your system setup.

When your node or nodes have started, run `vdash`, typically in a different terminal:
```sh
vdash ~/.local/share/safe/node/*/safenode.log
```
Note:

- `killall safenode` makes sure no existing nodes are still running, and
  deleting the `node` directory prevents you picking up statistics from previous logfiles. If you leave the logfile in place then `vdash` will waste time
  processing that, although you can skip that process using a command line option.

- setting SN_LOG ensures the logfiles contain the data which vdash needs, and
  excludes some that gets in the way.
- On Windows to set SN_LOG environment variable:

	Using Windows Command Line:
	```
	set SN_LOG="all"
	safenode
	```

	Using Windows PowerShell:
	```
	$env:SN_LOG="all"
	safenode
	```

### Using vdash With a Local Test Network

**IMPORTANT:** This section is out of date and so will not work as shown. You can try `vdash` by participating in one of the public test networks which are announced on the Safe Network [forum](https://safenetforum.org). These are happening about once per week during 2023.

1. **Start a local test network:** follow the instructions to [Run a local network](https://github.com/maidsafe/sn_api/tree/master/sn_cli#run-a-local-network), for example:
    ```sh
    rm -rf ~/.safe/node/local-test-network/
    cd safe_network
    killall safenode || true && SN_LOG=all cargo run --bin testnet -- -b --interval 100
    ```
    Windows: see "Note" immediately above for how to set SN_LOG on Windows.

2. **Run vdash:** in a different terminal window, start `vdash` with:
    You can then run `vdash`, typically in a different terminal:
    ```sh
    vdash ~/.safe/node/local-test-network/safenode-*/safenode.log
    ```
    Or with a live network:
    ```
    vdash ~/.safe/node/local-node/safenode.log
    ```
3. **Upload files using Safe CLI:** using the Safe CLI you can perform operations on the local test network that will affect the node and the effects will be shown in `vdash`. For example, to [use the Safe CLI to upload files](https://github.com/maidsafe/sn_api/tree/master/sn_cli#files):
    ```
    safe files put ./<some-directory>/ --recursive
    ```

If you want to try `vdash` with a live network, check to see if one is running at the Safe Network community forum: https://safenetforum.org

## Build

See [Get Safe Network Pre-requisites](#get-safe-network-pre-requisites).

### Get code
```
git clone https://github.com/happybeing/vdash
cd vdash
```

### Build - Linux / MacOS / Windows 10
Note: MacOS and Windows are untested but may 'just work' - please report success or failure in an issue.
```
cargo build --release
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
Note: the above figures are out of date but illustrate the point.

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
  - [x] track safenode [issue #1126](https://github.com/maidsafe/safenode/issues/1126) (maintain Get/Put response in)
  - [x] implement storage 'meter'
    - [x] code to get node storage used
    - [x] code to get free space on same device
    - [x] implement storage used/free 'progress' bar
  - [x] implement bandwidth 'meter'
    - [x] code to get node bandwidth
    - [x] code to get total bandwidth
    - [ ] implement bandwidth node/total/max in last day 'progress' bar
- [x] Implement DashOverview: all nodes on one page (rename from DashSummary)
- [x] trim NodeMetrics timeline
- [x] Implement popup help on ?, h, H
- [x] FIXED by upate to tui-rs v0.11.0 [Issue #382](https://github.com/fdehau/tui-rs/issues/382): Window titles corrupted when using --debug-window
- [x] switch to crossterm only (v0.9.0)
- [x] port from tui-rs (deprecated) to ratatui (supported fork of tui-rs)
- [x] Ability to provide 'glob' paths and re-scan them to add new nodes while running
- [ ] Implement logfile checkpoints to allow re-starting `vdash` quickly, and without losing data

## LICENSE

Everything is GPL3.0 unless otherwise stated. Any contributions are accepted on the condition they conform to this license.

See also [./LICENSE](./LICENSE)