# Terminal Dashboard for Monitoring Log Files

**logtail-dash** is a Rust command line program which displays the last few lines of a one or more logfiles in the terminal. It watches for changes and updates the display in the manner of `tail -f`. 

The command is written in Rust and uses [tui-rs](https://github.com/fdehau/tui-rs) to create the terminal UI, and [linemux](https://github.com/jmagnuson/linemux) to monitor the logfiles.

It is not particularly clever and was written as a learning project, but is a useful little utility which I believe could easily be adapted for MacOS or Windows.

Starting from **logtail-dash** I'm working on a SAFE Network Vault Dashboard ([vault-dash](https://github.com/theWebalyst/vault-dash)) which will provide metrics based on vault logfiles. In fact **vault-dash** was the original goal but I saw value in splitting out **logtail-dash** as a separate utility.

## Usage:

In the terminal type the command and the paths of one or more logfiles you want to monitor. For example:

    logtail /var/log/auth.log /var/log/kern.log

When the dashboard is active, pressing 'v' or 'h' switches between horizontal and vertical arrangments (when vieing more than one logfile).

For more information:

    logtail --help

## Build
### Get pre-requisites
1. **Get Rust:** see: https://doc.rust-lang.org/cargo/getting-started/installation.html

### Build logtail-dash
```
git clone https://github.com/theWebalyst/logtail-dash
cd logtail-dash
cargo build
```

### Quick Test
Here's a couple of useful commands to build and run logtail-dash using Linux logfiles rather than actual vault files. 

Open two terminals and in one run logtail-dash with:
```
RUSTFLAGS="$RUSTFLAGS -A unused" cargo run /var/log/auth.log /var/log/kern.log
```

In a second terminal you can affect the first logfile by trying and failing to 'su root':
```
su root </dev/null
```

You can use any logfiles for this basic level of testing.

## LICENSE

Everything is GPL3.0 unless otherwise stated. Any contributions are accepted on the condition they conform to this license.

See also ./LICENSE