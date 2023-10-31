///! Simple status message
///!

use chrono::{DateTime, Utc, Duration};

pub struct StatusMessage {
	pub current_message: Option<String>,
	pub default_duration: Option<Duration>,
	pub default_message: String,

	clear_at_time: Option<DateTime<chrono::Utc>>,
}

impl StatusMessage {
	pub fn new(default_message: &String, default_duration: &Duration) -> StatusMessage {
		StatusMessage {
			current_message: None,
			default_duration: Some(*default_duration),
			default_message: String::from(default_message),
			clear_at_time: None,
		}
	}

	pub fn set_status(&mut self, new_message: &String, new_duration: Option<Duration>) {
		self.current_message = Some(String::from(new_message));
		let duration = if let Some(duration) = new_duration {
			Some(duration) } else { self.default_duration };

		self.clear_at_time = match duration {
			Some(duration) => Some(Utc::now() + duration),
			None => None,
		};
	}

	pub fn get_status(&mut self) -> String {
	 	if let Some(expiry_time) = self.clear_at_time {
			if expiry_time.lt(&Utc::now()) {
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