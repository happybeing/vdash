///! Simple status message
///!

use chrono::{DateTime, Utc, Duration};

pub struct StatusMessage {
	pub current_message: Option<String>,
	pub default_duration: Duration,
	pub default_message: String,

	clear_at_time: Option<DateTime<chrono::Utc>>,
	to_console: bool,
}

/// Send a status message to the console, or store it for display with a duration (e.g. by terminal GUI)
impl StatusMessage {
	pub fn new(default_message: &String, default_duration: &Duration) -> StatusMessage {
		StatusMessage {
			current_message: None,
			default_duration: *default_duration,
			default_message: String::from(default_message),
			clear_at_time: None,
			to_console: true,
		}
	}

	fn reset(&mut self) {
		*self = StatusMessage::new(&self.default_message, &self.default_duration);
	}

	pub fn disable_to_console(&mut self) {	self.reset(); self.to_console = false; }
	pub fn enable_to_console(&mut self) {	self.to_console = true; }

	pub fn message(&mut self, new_message: &String, new_duration: Option<Duration>) {
		if self.to_console { eprintln!("{}", new_message); }
		self.current_message = Some(String::from(new_message));

		let duration = if let Some(duration) = new_duration {
			Some(duration) } else { Some(self.default_duration) };

		self.clear_at_time = match duration {
			Some(duration) => Some(Utc::now() + duration),
			None => None,
		};
	}

	pub fn clear_status(&mut self) { self.current_message = None; }

	pub fn get_status(&mut self) -> String {
	 	if let Some(expiry_time) = self.clear_at_time {
			if Utc::now() > expiry_time {
				self.current_message = None;
				self.clear_at_time = None;
			};
		};

		match &self.current_message {
			Some(string) => &string,
			None => &self.default_message,
		}.clone()
	}
}