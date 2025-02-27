///! Command line options and usage

pub static MIN_TIMELINE_STEPS: usize = 10;

pub use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
	about = "Monitor Autonomi Network nodes in the terminal.\nNavigate using tab and arrow keys."
)]
pub struct Opt {
	/// Maximum number of lines to display for each logfile
	#[structopt(short = "l", long, default_value = "100")]
	pub lines_max: usize,

	/// Event update tick in milliseconds (controls screen refresh rate)
	#[structopt(long, default_value = "200")]
	pub tick_rate: u64,

	/// Steps in each timeline for timeline graphs the Node Status display. Timeline 'width' = (steps * time units).
	#[structopt(short, long, default_value = "210")]
	pub timeline_steps: usize,

	/// Ignore any existing logfile content
	#[structopt(short, long)]
	pub ignore_existing: bool,

	/// A *nix 'glob' path to match multiple files.
	/// Can be provided multiple times as here:
	///
	///   vdash -g "$HOME/.local/share/autonomi/node/**/antnode.log" -g "./remote-node-logs/*/logs/antnode.log"
	#[structopt(name = "glob-path", short, long, multiple = true)]
	pub glob_paths: Vec<String>,

	/// Enable periodic scan of any glob paths every so many seconds. 0 to disable.
	#[structopt(long, default_value = "0")]
	pub glob_scan: i64,

	/// Set checkpoint interval in seconds (0 will disable checkpoints). vdash saves node statistics every few seconds so that it doesn't lose data when restarted.
	#[structopt(long, default_value = "300")]
	pub checkpoint_interval: u64,

	/// Token conversion rate as a positive floating point number (e.g. 3.345)
	/// This will be used if the price APIs are not used or failing.
	#[structopt(long, default_value = "-1")]
	pub currency_token_rate: f64,

	/// Fiat currency name for API
	#[structopt(long, default_value = "USD")]
	pub currency_apiname: String,

	/// Single character symbol for currency (e.g. "£" or "€")
	#[structopt(long, default_value = "$")]
	pub currency_symbol: String,

	/// Coingecko.com API key
	#[structopt(long)]
	pub coingecko_key: Option<String>,

	/// Coingecko.com API polling interval (minutes)
	#[structopt(long, default_value = "30")]
	pub coingecko_interval: usize,

	/// Coinmarketcap.com API key
	#[structopt(long)]
	pub coinmarketcap_key: Option<String>,

	/// Coinmarketcap.com API polling interval (minutes)
	#[structopt(long, default_value = "30")]
	pub coinmarketcap_interval: usize,

	/// One or more logfiles to monitor
	#[structopt(name = "LOGFILE")]
	pub files: Vec<String>,

	/// Parses first logfile *only* and adds a debug output window (accessed with l/r arrow)
	/// Also shows smaller debug output window to the right of the node view for the logfile
	#[structopt(short, long)]
	pub debug_window: bool,
}

pub fn get_app_name() -> String {
	String::from(Opt::clap().get_name())
}
pub fn get_app_version() -> String {
	String::from(structopt::clap::crate_version!())
}
