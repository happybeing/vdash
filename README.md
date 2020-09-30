# SAFE Network Vault Dashboard

`vdash` is a SAFE Network Vault dashboard for the terminal. It is written in Rust, using [tui-rs](https://github.com/fdehau/tui-rs) to create the terminal UI and [linemux](https://github.com/jmagnuson/linemux) to monitor vault logfiles on the local machine.

**Status:** early but useful for real time monitoring or post run logfile analysis.

<img src="./screenshots/vdash v0.2.0.png" alt="screenshot of vdash v0.2.0">

## Specification
Feature requests and discussion are currently summarised in the opening post of the Safe Network forum topic: [Vault Dashboard ideas please!](https://safenetforum.org/t/vault-dashboard-ideas-please/32572?u=happybeing) 
## TODO
Where `vdash` is headed:
- [x] implement ability to parse logfiles
  - [x] add --debug-parser to show results in second logfile
  - [x] implement parsing log file for simple metrics and timeline
  - [x] keep the debug UI available (selected with 'D' when using --debug-parse)
- [x] change events to use tokio mpsc (unbounded) channel
- [x] does tokio mpsc fix loss of updates from linemux (see linemux [issue #17](https://github.com/jmagnuson/linemux/issues/17))
- [ ] implement vault dashboard
  - [x] vault status summary page (single vault)
  - [x] debug window (--debug-window)
  - [x] add basic vault stats (age/PUTs/GETs)
  - [x] scroll vault logfile (arrow keys)
  - [x] multiple vaults (navigate with tab and arrow keys)
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
  - [ ] implement storage 'meter'
    - [ ] code to get vault storage used
    - [ ] code to get free space on same device
    - [ ] implement storage used/free 'progress' bar
  - [ ] implement bandwidth 'meter'
    - [ ] code to get vault bandwidth
    - [ ] code to get total bandwidth
    - [ ] implement bandwidth vault/total/max in last day 'progress' bar
- [ ] Implement DashOverview: all vaults on one page (rename from DashSummary)
- [ ] trim VaultMetrics timeline
- [ ] logtail-dash [Issue #1](https://github.com/theWebalyst/logfile-dash/issues/1): Implement popup help on ?, h, H
- [x] FIXED by upate to tui-rs v0.11.0 [Issue #382](https://github.com/fdehau/tui-rs/issues/382): Window titles corrupted when using --debug-window
- [ ] Implement --features="vdash" / --features="logtail" to select app and UI

## Operating Systems
- **Linux:** works on Ubuntu.
- **Windows:** works on Windows 10.
- **MacOS:** let me know what happens!

## Install from crates.io

1 Install **Rust** via https://doc.rust-lang.org/cargo/getting-started/installation.html

2a. **Linux/MacOS** install **vdash:**

    cargo install vdash
    vdash --help

2b. **Windows** install **vdash-crossterm:**

    cargo install vdash --bin logtail-crossterm --features="crossterm"
    vdash-crossterm --help

3a. **Linux/MacOS** (optional) install **vdash:**

    cargo install vdash
    vdash --help

3b. **Windows** (optional) install **vdash-crossterm:**

    cargo install vdash --bin vdash-crossterm --features="crossterm"
    vdash-crossterm --help

## Using vdash - SAFE Network Vault Dashboard
`vdash` is provides a terminal based graphical dashboard of SAFE Network Vault activity on the local machine. It parses input from one or more vault logfiles to gather live vault metrics which are displayed using terminal graphics.

**Status:** work-in-progress, not useful yet unless you want to help!

## Get SAFE Network pre-requisites
1. **Get Rust:** see: https://doc.rust-lang.org/cargo/getting-started/installation.html

2. **Get the SAFE CLI:** either download using an install script or build the SAFE CLI locally. Instructions for both options are [here](https://github.com/maidsafe/sn_api/tree/master/sn_cli#safe-network-cli).

3. **Get the SAFE Vault:** when you have the SAFE CLI working you can install the vault software with the command ` safe vault install` (details [here](https://github.com/maidsafe/sn_api/tree/master/sn_cli#vault-install)).

You are now ready to install `vdash` and can test it by running a local test network.

## Usage

In the terminal type the command and the paths of one or more vault logfiles you want to monitor. For example:

    vdash ~/.safe/vault/local-vault/safe_vault.log

When the dashboard is active, pressing 's' or 'd' switches between summary and detail views.
For more information:

    vdash --help

### Vault Setup
**IMPORTANT:** You must ensure the vault logfile includes the telemetry information used by vdash by setting the required logging level (e.g. 'info', or 'debug' etc).

The required level may change as things progress, so for now I recommend using a logging level of 'info' to keep resources minimal. The logfile will be larger and **vdash** become slower, but may have access to more metrics if you increase the logging level to 'debug', or even 'trace'.

You control the vault logging level by setting the environment variable `RUST_LOG` but be aware that setting this to one of  to one of 'warn', 'info', 'debug', or 'trace' will apply this to *all* modules used by `safe_vault` code, not just the `safe_vault` module. You can though set the default to one level and different levels for other modules.

For example, to set the default level to 'debug' for everything, except for the `quinn` module which generates a lot of unnecessary INFO log messages, module use:

```sh
safe vault killall
rm -f ~/.safe/vault/local-vault/safe_vault.log
RUST_LOG=debug,quinn=error safe vault join
```
Or
```sh
safe vault killall
rm -f ~/.safe/vault/baby-fleming-vaults/*/safe_vault.log
RUST_LOG=debug,quinn=error safe vault run-baby-fleming -t
```
Note:
- `save vault killall` makes sure no existing vaults are still running, and deleting existing logfiles prevents you picking up statistics from previous activity. If you leave the logfile in place then `vdash` will waste time processing that, although you can skip that process using a command line option.
- setting RUST_LOG ensures the logfiles contain the data which vdash needs, and excludes some that gets in the way.
When there is a live test network you will be able to use `vdash` with that, but pre-beta those test networks are only available intermittently. The following therefore shows how to run a local test network and use `vdash` with this.

### Using vdash With a Local Test Network
1. **Start a local test network:** follow the instructions to [Run a local network](https://github.com/maidsafe/sn_api/tree/master/sn_cli#run-a-local-network), but I suggest using the `-t` option to create an account and authorise the CLI with it altogether. As here:
    ```
    safe vault killall
    rm -f ~/.safe/vault/baby-fleming-vaults/*/safe_vault.log
    RUST_LOG=debug,quinn=error safe vault run-baby-fleming -t
    ```
2. **Run vdash:** in a different terminal window (so you can continue to use the safe-cli in the first terminal), start `vdash` with:
    ```
    vdash ~/.safe/vault/baby-fleming-vaults/*/safe_vault.log
    ```
    Or with a live network:
    ```
    vdash ~/.safe/vault/local-vault/safe_vault.log
    ```
3. **Upload files using SAFE CLI:** in the SAFE CLI window you can perform operations on the local test network that will affect the vault and the effects will be shown in `vdash`. For example, to [use the SAFE CLI to upload files](https://github.com/maidsafe/sn_api/tree/master/sn_cli#files):
    ```
    safe files put ./<some-directory>/ --recursive
    ```

If you want to try `vdash` with a live network, check to see if one is running at the SAFE Network community forum: https://safenetforum.org

## Build

See [Get SAFE Network Pre-requisites](#get-safe-network-pre-requisites).

### Get code
```
git clone https://github.com/theWebalyst/vdash
cd vdash
```

### Build

#### Linux / MacOS
Build `vdash` with the termion backend (see [tui-rs](https://github.com/fdehau/tui-rs)).
Note: MacOS is untested but may 'just work'.
```
cargo build --features="termion" --features="vdash" --release
```

#### Windows 10
Builds `vdash` the crossterm backend (see [tui-rs](https://github.com/fdehau/tui-rs)), with the intention to support Windows.

NOT working on Windows yet, this is being worked on at the moment. Help with testing appreciated.
```
cargo build --bin logtail-crossterm --features="crossterm" --features="vdash" --release
```

## LICENSE

Everything is GPL3.0 unless otherwise stated. Any contributions are accepted on the condition they conform to this license.

See also ./LICENSE