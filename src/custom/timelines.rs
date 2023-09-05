use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};


///! Maintains one or more 'marching bucket' histories for
///! a given metric, each with its own duration and granularity.
///!
///! A Buckets is used to hold the history of values with
///! a given bucket_duration and maximum number of buckets.
///!
///! A Buckets begins with a single bucket of fixed
///! duration holding the initial metric value. New buckets
///! are added as time progresses until the number of buckets
///! covers the total duration of the Buckets. At this
///! point the oldest bucket is removed when a new bucket is
///! added, so that the total duration remains constant and
///! the specified maximum number of buckets is never
///! exceeded.
///!
///! By adding more than one Buckets, a given metric can be
///! recorded for different durations and with different
///! granularities. E.g. 60 * 1s buckets covers a minute
///! and 60 * 1m buckets covers an hour, and so on.
///!
///! TimelineMMM and BucketsMMM are similar structs which
///! implement timelines of min, mean and max values for
///! a given metric.

#[derive(Default)]
pub struct Timeline {
	name: String,
	buckets: HashMap<&'static str, Buckets>,
}

pub struct Buckets {
	pub bucket_time: Option<DateTime<Utc>>,
	pub total_duration: Duration,
	pub bucket_duration: Duration,
	pub max_buckets: usize,
	pub buckets: Vec<u64>,
}

impl Timeline {
	pub fn new(name: String) -> Timeline {
		Timeline {
			name,
			buckets: HashMap::<&'static str, Buckets>::new(),
		}
	}

	pub fn get_name(&self) -> &String {
		&self.name
	}

	pub fn add_bucket_set(&mut self, name: &'static str, duration: Duration, max_buckets: usize) {
		self.buckets
			.insert(name, Buckets::new(duration, max_buckets));
	}

	pub fn get_bucket_set(&mut self, bucket_set_name: &str) -> Option<&Buckets> {
		self.buckets.get(bucket_set_name)
	}

	///! Update all buckets with new current time
	///!
	///! Call significantly more frequently than the smallest Buckets duration
	pub fn update_current_time(&mut self, new_time: &DateTime<Utc>) {
		// debug_log!("update_current_time()");
		for (_name, bs) in self.buckets.iter_mut() {
			if let Some(mut bucket_time) = bs.bucket_time {
				let mut end_time = bucket_time + bs.bucket_duration;
				// debug_log!(format!("end_time       : {}", end_time).as_str());

				while end_time.lt(&new_time) {
					// debug_log!("Start new bucket");
					// Start new bucket
					bs.bucket_time = Some(end_time);
					bucket_time = end_time;
					end_time = bucket_time + bs.bucket_duration;

					bs.buckets.push(0);
					if bs.buckets.len() > bs.max_buckets {
						bs.buckets.remove(0);
					}
				}
			} else {
				bs.bucket_time = Some(*new_time);
			}
		}
	}

	pub fn increment_value(&mut self, time: &DateTime<Utc>) {
		// debug_log!("increment_value()");
		for (_name, bs) in self.buckets.iter_mut() {
			// debug_log!(format!("name       : {}", _name).as_str());
			let mut index = Some(bs.buckets.len() - 1);
			// debug_log!(format!("time       : {}", time).as_str());
			if let Some(bucket_time) = bs.bucket_time {
			// debug_log!(format!("bucket_time: {}", bucket_time).as_str());
				if time.lt(&bucket_time) {
					// Use the closest bucket to this time
					// debug_log!("increment (closest bucket)");
					let time_difference = (bucket_time - *time).num_nanoseconds();
					let bucket_duration = bs.bucket_duration.num_nanoseconds();
					if time_difference.and(bucket_duration).is_some() {
						let buckets_behind = time_difference.unwrap() / bucket_duration.unwrap();
						if buckets_behind as usize >= bs.buckets.len() {
							// debug_log!(format!("increment DISCARDED buckets_behind: {}", buckets_behind).as_str());
							index = None;
						} else {
							// debug_log!(format!("increment INCLUDED buckets_behind: {}", buckets_behind).as_str());
							index = Some(bs.buckets.len() - 1 - buckets_behind as usize);
						}
					}
				}
			}
			if let Some(index) = index {
				// debug_log!(format!("increment index: {}", index).as_str());
				bs.buckets[index] += 1;
			}
		}
	}
}

impl Buckets {
	pub fn new(bucket_duration: Duration, max_buckets: usize) -> Buckets {
		Buckets {
			bucket_duration,
			max_buckets,
			total_duration: bucket_duration * max_buckets as i32,

			bucket_time: None,
			buckets: vec![0; max_buckets],
		}
	}

	pub fn set_bucket_value(&mut self, value: u64) {
		let index = self.buckets.len() - 1;
		self.buckets[index] = value;
	}

	pub fn increment_value(&mut self) {
		let index = self.buckets.len() - 1;
		self.buckets[index] += 1;
	}

	pub fn buckets(&self) -> &Vec<u64> {
		&self.buckets
	}

	pub fn buckets_mut(&mut self) -> &mut Vec<u64> {
		&mut self.buckets
	}
}

