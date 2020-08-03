# Terminal Dashboard for a SAFE Network Vault

**Status:** experimental code, nothing to see here yet!

**safe-dash** is a Rust command line program which uses [tui-rs](https://github.com/fdehau/tui-rs) to display a dashboard based in the terminal, using data that arrives from an input stream (stdin). Initially for use with a SAFE Network Vault, it should be general enough to be used to provide a dashboard for any suitably formatted input stream.

The aims to provide a terminal based graphical dashboard display based of SAFE Network Vault status and activity for a vault on the local machine. **safe-dash** will consume input from stdin and use it to display display and update terminal graphics containing one or more customisable charts.

The plan is to leverage *nix command line tools to deliver suitable input to **safe-dash**, and allow simple charts to be selected and customised with command line options, with the option of using configuration files when things get more complex.

## TODO
- [x] make skeleton app which can parse command line options and display usage
- [ ] use tui-rs to show stdin in a scrolling 'Debug' window
  - [ ] make a window that scrolls text
  - [ ] grab text from stdin and send to the window
- [ ] add some charts
  - [ ] mock storage chart: horizontal bar chart (vault storage limit/used)
  - [ ] mock chunk metering: vertical bar chart (total, immutable, sequence etc chunks)
  - [ ] get data into storage chart (poll disk)
  - [ ] get data into chunk metering (initialise from log file, accumulate session from stdin)

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

### Test safe-dash
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