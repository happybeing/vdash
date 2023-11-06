///! Command line options and usage

pub static MIN_TIMELINE_STEPS: usize = 10;

pub use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
	about = "Monitor Safe Network nodes in the terminal.\nNavigate using tab and arrow keys."
)]
pub struct Opt {
	/// Maximum number of lines to keep for each logfile
	#[structopt(short = "l", long, default_value = "100")]
	pub lines_max: usize,

	/// Event update tick in milliseconds
	#[structopt(long, default_value = "200")]
	pub tick_rate: u64,

	/// Steps (width) of each timeline, helps tweak right justification.
	#[structopt(short, long, default_value = "210")]
	pub timeline_steps: usize,

	/// Ignore any existing logfile content
	#[structopt(short, long)]
	pub ignore_existing: bool,

	/// A *nix 'glob' path to match multiple files.
	/// Can be provided multiple times as here:
	///
	///    vdash -g '~/logfiles/*/safenode.log' -g '/home/user/.local/share/safe/node/**/safenode.log'
	#[structopt(name="glob-path", short, long, multiple=true)]
	pub glob_paths: Vec<String>,

	/// Enable periodic scan of any glob paths every so many seconds. 0 to disable.
	#[structopt(long, default_value = "0")]
	pub glob_scan: i64,

	/// One or more logfiles to monitor
	#[structopt(name = "LOGFILE")]
	pub files: Vec<String>,

	/// Parses first logfile *only* and adds a debug output window (accessed with l/r arrow)
	/// Also shows smaller debug output window to the right of the node view for the logfile
	#[structopt(short, long)]
	pub debug_window: bool,
}

pub fn get_app_name() -> String { String::from(Opt::clap().get_name()) }
pub fn get_app_version() -> String { String::from(structopt::clap::crate_version!()) }
