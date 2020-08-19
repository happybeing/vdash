# Terminal Dashboard for Monitoring Log Files

**logtail** is a Rust command line program which displays the last few lines of a one or more logfiles in the terminal. It watches for changes and updates the display in the manner of `tail -f`. 

The command is written in Rust and uses [tui-rs](https://github.com/fdehau/tui-rs) to create the terminal UI, and [linemux](https://github.com/jmagnuson/linemux) to monitor the logfiles.

It is not particularly clever and was written as a learning project, but is a useful little utility.

Supports **Linux** only as far as I know, but it's worth testing out on **MacOS** and **Windows**. If it doesn't work I don't think it will be hard to get working on those platforms.

Starting from **logtail** I'm working on a SAFE Network Vault Dashboard ([vault-dash](https://github.com/theWebalyst/vault-dash)) which will provide metrics based on vault logfiles. In fact **vault-dash** was the original goal but I saw value in splitting out **logtail-dash** as a separate utility.

## Install from crates.io

    cargo install logtail

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

#### Linux / MacOS (logtail-termion)
Builds logtail-termion which uses the termion backend (see [tui-rs](https://github.com/fdehau/tui-rs)).
Note: MacOS is untested
```
cargo build --bin logtail-termion --features="termion" --release
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