# Terminal Dashboard for Monitoring Log Files

This repository contains several applications, each on its own branch. The branches and the commands they build are:
- **logtail-dash** : `logtail` will display one or more log files in the terminal in the manner of `tail -f`.

- **vault-dash** : `vault-dash` shows a SAFE Network Vault status dashboard in the terminal.

The commands are written in Rust, using [tui-rs](https://github.com/fdehau/tui-rs) to create the terminal UI and [linemux](https://github.com/jmagnuson/linemux) to monitor the logfiles.

## Operating Systems
`logtail`:
- **Linux:** works on Ubuntu.
- **MacOS:** may 'just work' but has not been tested - please do!
- **Windows:** is currently being tested, so feel free to check it out.

`vault-dash`:
-  is work in progress so watch the repo for updates if you would like to try it.
## How to Install from crates.io

### Pre-requisite
Install **Rust** via https://doc.rust-lang.org/cargo/getting-started/installation.html

Install **logtail:**

    cargo install --branch logtail-dash logtail

Install **vault-dash:**

    cargo install --branch vault-dash logtail

## logtail or vault-dash
The rest of this README relates only to **logtail-dash**, for more information of **vault-dash** switch to the README on that branch.

## Logtail
**logtail** is a Rust command line program which displays the last few lines of a one or more logfiles in the terminal. It watches for changes and updates the display in the manner of `tail -f`.

## Usage

In the terminal type the command and the paths of one or more logfiles you want to monitor. For example:

    logtail /var/log/auth.log /var/log/kern.log

When the dashboard is active, pressing 'v' or 'h' switches between horizontal and vertical arrangments (when viewing more than one logfile).

For more information:

    logtail --help

## Build
### Get pre-requisites
1. **Get Rust:** see: https://doc.rust-lang.org/cargo/getting-started/installation.html

### Get code
```
git clone https://github.com/theWebalyst/logtail-dash
cd logtail-dash
```

### Build

#### Linux / MacOS (logtail)
Builds logtail which uses the termion backend (see [tui-rs](https://github.com/fdehau/tui-rs)).
Note: MacOS is untested
```
cargo build --bin logtail --features="termion" --release
```

#### Windows 10 (logtail-crossterm)
Builds logtail-crossterm which uses the crossterm backend (see [tui-rs](https://github.com/fdehau/tui-rs)), with the intention to support Windows.

NOT working on Windows yet, this is being worked on at the moment. Help with testing appreciated.
```
cargo build --bin logtail-crossterm --features="crossterm" --release
```

### Quick Test
Here's a couple of useful commands to build and run `logtail` to monitor a couple of Linux logfiles.

Open two terminals and in one run logtail-dash with:
```
RUSTFLAGS="$RUSTFLAGS -A unused" cargo run --bin logtail --features="termion"  /var/log/auth.log /var/log/kern.log
```

In a second terminal you can affect the first logfile by trying and failing to 'su root':
```
su root </dev/null
```

You can use any logfiles for this basic level of testing. Here are some to try:

    /var/log/syslog
    /var/log/auth.log 
    /var/log/lastlog
    /var/log/dmesg
    /var/log/kern.log
    /var/log/boot.log

## LICENSE

Everything is GPL3.0 unless otherwise stated. Any contributions are accepted on the condition they conform to this license.

See also ./LICENSE