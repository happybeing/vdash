///! Command line options and usage
///!
///! Edit src/custom/opt.rs to create a customised fork of logtail-dash

static MAX_CONTENT: &str = "100";

pub use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(about = "Monitor multiple logfiles in the terminal.")]
pub struct Opt {
	/// Maximum number of lines to keep for each logfile
	#[structopt(short = "l", long, default_value = "100")]
	pub lines_max: usize,

	/// Time between ticks in milliseconds
	#[structopt(short, long, default_value = "200")]
	pub tick_rate: u64,

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
