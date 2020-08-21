# SAFE Network Vault Dashboard

`vdash` is a SAFE Network Vault dashboard for the terminal. It is written in Rust, using [tui-rs](https://github.com/fdehau/tui-rs) to create the terminal UI and [linemux](https://github.com/jmagnuson/linemux) to monitor vault logfiles on the local machine.

**Status:** work in progress - not useful yet unless you want to help with testing.

## TODO
Where `vdash` is headed:
- [ ] logtail-dash [Issue #1](https://github.com/theWebalyst/logfile-dash/issues/1): Implement popup help on ?, h, H
- [ ] Implement --features="vdash" / --features="logterm" to select app and UI
- [ ] Summary view: all vaults on one page
  - [x] just logfile for each vault (divide vertically)
  - [ ] add a storage summary to the left of each logfile
- [ ] Detail view: tab for each vault
  - [ ] Indicate the current logfile (default to the first)
  - [ ] Create empty bands ready for..
  - [ ] h-band1: Heading of logfile and space for some metrics (e.g. size MB)
  - [ ] h-band2: Storage chart / Data Types chart h-bar
  - [ ] h-band3: Activity over time (full width)
  - [ ] h-band4: Logfile (full width)
- [ ] investigate removing tokio to just use standard runtime (see linemux [issue #15](https://github.com/jmagnuson/linemux/issues/15))
- [ ] add some charts
  - [ ] add parsing of dummy logfile input to LogMonitor
  - [ ] use to generate a dummy test chart
  - [ ] update parser to work on real vault log (keeping test logfile as an option)
  - [ ] mock storage chart: horizontal bar chart (vault storage limit/used)
  - [ ] mock chunk metering: vertical bar chart (total, immutable, sequence etc chunks)
  - [ ] get real data into storage chart (poll disk)
  - [ ] get real data into chunk metering

## Operating Systems
- **Linux:** builds on Ubuntu.
- **MacOS:** may 'just work' but has not been tested - please do!
- **Windows:** is currently being tested, so feel free to check it out.

## Install from crates.io

1 Install **Rust** via https://doc.rust-lang.org/cargo/getting-started/installation.html

2a. **Linux/MacOS** install **logtail:**

    cargo install logtail
    logtail-crossterm --help

2b. **Windows** install **logtail-crossterm:**

    cargo install logtail --bin logtail-crossterm --features="crossterm"    logtail-crossterm --help

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

2. **Get the SAFE CLI:** either download using an install script or build the SAFE CLI locally. Instructions for both options are [here](https://github.com/maidsafe/safe-api/tree/master/safe-cli#safe-cli).

3. **Get the SAFE Vault:** when you have the SAFE CLI working you can install the vault software with the command ` safe vault install` (details [here](https://github.com/maidsafe/safe-api/tree/master/safe-cli#vault-install)).

You are now ready to install `vdash` and can test it by running a local test network.

## Usage

In the terminal type the command and the paths of one or more vault logfiles you want to monitor. For example:

    vdash ~/.safe/vault/local-vault/safe_vault.log

When the dashboard is active, pressing 's' or 'd' switches between summary and detail views.
For more information:

    vdash --help

### Vault Test
When there is a live test network you will be able to use `vdash` with that, but pre-beta those test networks are only available intermittently. The following therefore shows how to run a local test network and use `vdash` with this.

1. **Start a local test network:** follow the instructions to [Run a local network](https://github.com/maidsafe/safe-api/tree/master/safe-cli#run-a-local-network), but I suggest using the `-t` option to create an account and authorise the CLI with it altogether. As here:
    ```
    safe vault -t run-baby-fleming
    ```
2. **Run vdash:** in a different terminal window (so you can continue to use the safe-cli in the first terminal), start `vdash` with:
    ```
    vdash ~/.safe/vault/local-vault/safe_vault.log
    ```
3. **Upload files using SAFE CLI:** in the SAFE CLI window you can perform operations on the local test network that will affect the vault and the effects will be shown in `vdash`. For example, to [use the SAFE CLI to upload files](https://github.com/maidsafe/safe-api/tree/master/safe-cli#files):
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