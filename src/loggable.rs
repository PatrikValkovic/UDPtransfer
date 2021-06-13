use time::OffsetDateTime;
use crate::DATE_FORMAT_STR;

///Trait to allow logging for the configs
pub trait Loggable {
    fn is_verbose(&self) -> bool;

    fn vlog(&self, text: &str) -> () {
        if self.is_verbose() {
            println!("{}: {}", OffsetDateTime::now_utc().format(DATE_FORMAT_STR),text);
        }
    }
}
