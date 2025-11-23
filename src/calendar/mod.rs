pub mod event;
pub mod calendar_type;

pub use event::{Event, EventStatus, Reminder, ReminderMethod};
pub use calendar_type::{Calendar, AccessRole};

pub const DEFAULT_CALENDAR_ID: &str = "primary";
