use std::collections::HashMap;
use chrono::{DateTime, Duration, Utc};
use tui::style::Color;


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

/// Specify min, mean, max series (as opposed to value series)
#[derive(Default)]
pub enum MinMeanMax {
	#[default]
    Min = 1,
    Mean = 2,
    Max = 3,
}

pub struct Timeline {
	pub name: String,
	pub is_mmm: bool,
	pub colour: Color,
	buckets: HashMap<&'static str, Buckets>,
}

impl Timeline {
	pub fn new(name: String, is_mmm: bool, colour: Color) -> Timeline {
		Timeline {
			name,
			is_mmm,
			buckets: HashMap::<&'static str, Buckets>::new(),
			colour,
		}
	}

	pub fn get_name(&self) -> &String {
		&self.name
	}

	pub fn add_bucket_set(&mut self, name: &'static str, duration: Duration, max_buckets: usize) {
		self.buckets
			.insert(name, Buckets::new(duration, max_buckets, self.is_mmm));
	}

	pub fn get_bucket_set(&mut self, timescale_name: &str) -> Option<&mut Buckets> {
		return self.buckets.get_mut(timescale_name);
	}

	pub fn get_buckets_mut(&mut self, timescale_name: &str, mmm_ui_mode: Option<&MinMeanMax>) -> Option<&Vec<u64>> {
		if let Some(bucket_set) = self.buckets.get(timescale_name) {
			return Some(bucket_set.buckets(mmm_ui_mode));
		} else {
			return None;
		}
	}

	pub fn get_buckets(&self, timescale_name: &str, mmm_ui_mode: Option<&MinMeanMax>) -> Option<&Vec<u64>> {
		if let Some(bucket_set) = self.buckets.get(timescale_name) {
			return Some(bucket_set.buckets(mmm_ui_mode));
		} else {
			return None;
		}
	}

	///! Update all Buckets with new current time
	///!
	///! Call significantly more frequently than the smallest Buckets duration
	pub fn update_current_time(&mut self, new_time: &DateTime<Utc>) {
		// debug_log!("update_current_time()");
		for (_name, bs) in self.buckets.iter_mut() {
			bs.update_current_time(new_time);
		}
	}

	pub fn increment_value(&mut self, time: &DateTime<Utc>) {
		self.add_value(time, 1);
	}

	pub fn add_value(&mut self, time: &DateTime<Utc>, value: u64) {
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
				bs.bucket_add_value(index, value);
			}
		}
	}
}

/// Buckets operate as a value series (e.g. count per bucket), or
/// if Some(stats_mmm) they maintain min, mean and max series.

// I use the same impl code for is_mmm true or false to avoid polymorphic code
pub struct Buckets {
	pub bucket_time: Option<DateTime<Utc>>,
	pub total_duration: Duration,
	pub bucket_duration: Duration,
	pub max_buckets: usize,
	pub is_mmm: bool,

	// if !is_mmm we only use buckets
	pub buckets: Vec<u64>,		// Value series

	// if is_mmm use only the following
	pub buckets_count: Vec<u64>,		// Number of values added to a bucket (timeslot)
	pub buckets_total: Vec<u64>,		// Total of all values added to a given bucket (timeslot)
	pub buckets_min: Vec<u64>,			// Min of all values
	pub buckets_mean: Vec<u64>,			// Average
	pub buckets_max: Vec<u64>,			// Max

	pub buckets_need_init: Vec<u64>,	// Filled with 1 and set to 0 after init
}

impl Buckets {
	pub fn new(bucket_duration: Duration, max_buckets: usize, is_mmm: bool) -> Buckets {
		let value_buckets_size =  if is_mmm { 0 } else { max_buckets };
		let mmm_buckets_size =  if is_mmm { max_buckets } else { 0 };

		return Buckets {
			bucket_duration,
			max_buckets,
			total_duration: bucket_duration * max_buckets as i32,

			bucket_time: None,

			is_mmm: is_mmm,
			buckets: vec![0; value_buckets_size],

			buckets_count: vec![0; mmm_buckets_size],
			buckets_total: vec![0; mmm_buckets_size],
			buckets_min: vec![0; mmm_buckets_size],
			buckets_mean: vec![0; mmm_buckets_size],
			buckets_max: vec![0; mmm_buckets_size],

			buckets_need_init: vec![1; mmm_buckets_size],
		}
	}

	/// Update all buckets with current time
	pub fn update_current_time(&mut self, new_time: &DateTime<Utc>) {
		if let Some(mut bucket_time) = self.bucket_time {
			let mut end_time = bucket_time + self.bucket_duration;
			// debug_log!(format!("end_time       : {}", end_time).as_str());
			while end_time.lt(&new_time) {
				// debug_log!("Start new bucket");
				// Start new bucket
				self.bucket_time = Some(end_time);
				bucket_time = end_time;
				end_time = bucket_time + self.bucket_duration;

				if self.is_mmm {
					for buckets in
						&mut vec![
							&mut self.buckets_count,
							&mut self.buckets_total,
							&mut self.buckets_min,
							&mut self.buckets_mean,
							&mut self.buckets_max].iter_mut() {

						buckets.push(0);
						if buckets.len() > self.max_buckets {
							buckets.remove(0);
						}
																}
					self.buckets_need_init.push(1);
					if self.buckets_need_init.len() > self.max_buckets {
						self.buckets_need_init.remove(0);
					}

				} else  {
					self.buckets.push(0);
					if self.buckets.len() > self.max_buckets {
						self.buckets.remove(0);
					}
				}
			}
			} else {
			self.bucket_time = Some(*new_time);
		}

	}

	pub fn bucket_add_value(&mut self, index: usize, value: u64) {
		if self.is_mmm {
			if self.buckets_need_init[index] == 1  {
				self.buckets_need_init[index] = 0;

				self.buckets_count[index] = 0;
				self.buckets_total[index] = 0;
				self.buckets_min[index] = u64::MAX;
				self.buckets_mean[index] = 0;
				self.buckets_max[index] = 0;
			}
			self.buckets_count[index] += 1;
			self.buckets_total[index] += value;
			self.buckets_mean[index] = self.buckets_total[index] / self.buckets_count[index];

			if value < self.buckets_min[index] { self.buckets_min[index] = value }
			if value > self.buckets_max[index] { self.buckets_max[index] = value }
		} else {
			self.buckets[index] += value;
		}
	}

	pub fn buckets(&self, mmm_ui_mode: Option<&MinMeanMax>) -> &Vec<u64> {
		if self.is_mmm {
			return match mmm_ui_mode {
				None => &self.buckets,
				Some(MinMeanMax::Min) => &self.buckets_min,
				Some(MinMeanMax::Mean) => &self.buckets_mean,
				Some(MinMeanMax::Max) => &self.buckets_max,
			}
		} else {
			return &self.buckets;
		}
	}
}
