# Terminal Dashboard for Monitoring Log Files

`logtail` displays one or more log files in the terminal in the manner of `tail -f`.

The command is written in Rust and uses [tui-rs](https://github.com/fdehau/tui-rs) to create the terminal UI, and [linemux](https://github.com/jmagnuson/linemux) to monitor the logfiles.

Note: `vdash` is a fork of `logtail` that provides a dashboard for SAFE Network Vaults (see [vdash](https://github.com/theWebalyst/vdash)).


## Operating Systems
- **Linux:** works on Ubuntu.
- **MacOS:** may 'just work' but has not been tested - please do!
- **Windows:** is currently being tested, so feel free to check it out.

## Install from crates.io
1. Install **Rust** via https://doc.rust-lang.org/cargo/getting-started/installation.html

2. Install **logtail:**

    cargo install logtail

3. Install (optional) **vdash:**

    cargo install vdash

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

#### Linux / MacOS
Builds logtail which uses the termion backend (see [tui-rs](https://github.com/fdehau/tui-rs)).
Note: MacOS is untested
```
cargo build --bin logtail --features="termion" --release
```

#### Windows 10
Builds logtail-crossterm which uses the crossterm backend (see [tui-rs](https://github.com/fdehau/tui-rs)), with the intention to support Windows.

NOT working on Windows yet, this is being worked on at the moment. Help with testing appreciated.
```
cargo build --bin logtail-crossterm --features="crossterm logtail" --release
```

### Quick Test
Here's a couple of useful commands to build and run `logtail` to monitor a couple of Linux logfiles.

Open two terminals and in one run logtail-dash with:
```
RUSTFLAGS="$RUSTFLAGS -A unused" cargo run --bin logtail --features="termion logtail"  /var/log/auth.log /var/log/kern.log
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