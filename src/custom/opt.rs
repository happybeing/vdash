///! Command line options and usage
///!
///! Edit src/custom/opt.rs to create a customised fork of logtail-dash

static MAX_CONTENT: &str = "100";

pub use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(about = "Monitor multiple logfiles in the terminal.")]
pub struct Opt {
  /// Maximum number of lines to keep for each logfile
  #[structopt(short = "l", long, default_value = MAX_CONTENT)]
  pub lines_max: usize,

  /// Ignore any existing logfile content
  #[structopt(short, long)]
  pub ignore_existing: bool,

  /// One or more logfiles to monitor
  #[structopt(name = "LOGFILE")]
  pub files: Vec<String>,
}

