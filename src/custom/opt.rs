///! Command line options and usage
///!
///! Edit src/custom/opt.rs to create a customised fork of logtail-dash

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

	/// One or more logfiles to monitor
	#[structopt(name = "LOGFILE")]
	pub files: Vec<String>,

	/// Show a debug window to the right of the logfile view in main dashboard
	#[structopt(short, long)]
	pub debug_window: bool,

	/// Parses first logfile, prints results to second and shows side-by-side (logtail-crossterm only)
	#[structopt(long)]
	pub debug_dashboard: bool,
}
