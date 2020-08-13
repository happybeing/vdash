# Terminal Dashboard for a SAFE Network Vault

**Status:** simple logfile viewing capability (branch: general-logile-viewer)

**safe-dash** is a Rust command line program which uses [tui-rs](https://github.com/fdehau/tui-rs) to display a dashboard based in the terminal, gathered from one or more logfiles, updated as each logfile grows. 

Although designed for use with a SAFE Network Vault, it should be easily adapted to create a dashboard for logfiles which can be parsed to gather metrics.

## SAFE Network Vault Dashboard
**safe-dash** aims to provide a terminal based graphical dashboard display based of SAFE Network Vault status and activity for a vault on the local machine. It parses input from one or more vault logfiles to gather live vault metrics which are displayed using terminal graphics.

## TODO
- [x] make skeleton app which can parse command line options and display usage
- [ ] use tui-rs to 'tail' specified logfiles in separate windows
  - [x] watch one or more logfiles specified on the command line
  - [x] send text for each logfile to its own window
  - [ ] make a window that scrolls text
  - [ ] add CLI param to specify number of logfile lines to scroll (remember ASSCROLL!)
- [x] implement events
  - [x] keyboard events: q = quit
  - [x] resize terminal window
  - [x] make simultaneous with logfile monitoring
- [x] Implement tabbing for Summary / Detail views
- [ ] [Issue #1](https://github.com/theWebalyst/safe-dash/issues/1https://github.com/theWebalyst/safe-dash/issues/1): Implement popup help on ?, h, H
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

## Build
### Get pre-requisites
1. **Get Rust:** see: https://doc.rust-lang.org/cargo/getting-started/installation.html

2. **Get the SAFE CLI:** either download using an install script or build the SAFE CLI locally. Instructions for both options are [here](https://github.com/maidsafe/safe-api/tree/master/safe-cli#safe-cli).

3. **Get the SAFE Vault:** when you have the SAFE CLI working you can install the vault software with the command ` safe vault install` (details [here](https://github.com/maidsafe/safe-api/tree/master/safe-cli#vault-install)).

You are now ready to get safe-dash and can test it by running a local test network as described next.

### Build safe-dash
```
git clone https://github.com/theWebalyst/safe-dash
cd safe-dash
cargo build
```

### Quick Test
Here's a couple of useful commands to build and run safe-dash using Linux logfiles rather than actual vault files. 

Open two terminals and in one run safe-dash with:
```
RUSTFLAGS="$RUSTFLAGS -A unused" cargo run /var/log/auth.log /var/log/apport.log  
```

In a second terminal you can affect the first logfile by trying and failing to 'su root':
```
su root </dev/null
```

You can use any logfiles for this basic level of testing.

### Vault Test
When there is a live test network you will be able to use safe-dash with that, but pre-beta those test networks are only available intermittently. The following therefore shows how to run a local test network and use safe-dash with this.

1. **Start a local test network:** follow the instructions to [Run a local network](https://github.com/maidsafe/safe-api/tree/master/safe-cli#run-a-local-network), but I suggest using the `-t` option to create an account and authorise the CLI with it altogether. As here:
    ```
    safe vault -t run-baby-fleming
    ```
2. **Run safe-dash:** in a different terminal window (so you can continue to use the safe-cli in the first terminal), start safe-dash with:
    ```
    cd safe-dash
    cargo run <params>
    ```
3. **Upload files using SAFE CLI:** in the SAFE CLI window you can perform operations on the local test network that will affect the vault and the effects will be shown in safe-dash. For example, to [use the SAFE CLI to upload files](https://github.com/maidsafe/safe-api/tree/master/safe-cli#files):
    ```
    safe files put ./<some-directory>/ --recursive
    ```

If you want to try safe-dash with a live network, check to see if one is running at the SAFE Network community forum: https://safenetforum.org

### safe-dash usage:
safe-dash `<params>` are still to be defined, but for now assume a path to the logfile of a running safe-vault.

## LICENSE

Everything is GPL3.0 unless otherwise stated. Any contributions are accepted on the condition they conform to this license.

See also ./LICENSE