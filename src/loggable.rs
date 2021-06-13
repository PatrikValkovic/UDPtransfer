///Trait to allow logging for the configs
pub trait Loggable {
    fn vlog(&self, content: &str) -> ();
}
