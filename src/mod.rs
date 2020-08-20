pub mod util;

#[cfg(feature = "termion")]
pub mod event;
#[cfg(feature = "termion")]
pub use event::{Event, Events};
