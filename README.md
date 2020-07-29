# Terminal Dashboard for a SAFE Network Vault

Status: experimental code, nothing to see here yet!

**safe-dash** is a Rust command line program which uses [tui-rs](https://github.com/fdehau/tui-rs) to display a dashboard based in the terminal, using data that arrives from an input stream (stdin). Initially for use with a SAFE Network Vault, it should be general enough to be used to provide a dashboard for any suitably formatted input stream.

The aims to provide a terminal based graphical dashboard display based of SAFE Network Vault status and activity for a vault on the local machine. **safe-dash** will consume input from stdin and use it to display display and update terminal graphics containing one or more customisable charts.

The plan is to leverage *nix command line tools to deliver suitable input to **safe-dash**, and allow simple charts to be selected and customised with command line options, with the option of using configuration files when things get more complex.

## TODO
- [x] make skeleton app which can parse command line options and display usage
- [ ] use tui-rs to show stdin in a scrolling 'Debug' window
- [ ] add a dummy chart
- [ ] animate the chart as new data arrives on stdin
- [ ] get a trivial display that updates as the vault log grows.

## LICENSE

Everything is GPL3.0 unless otherwise stated. Any contributions are accepted on the condition they conform to this license.

See also ./LICENSE