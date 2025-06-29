use crate::event::Event;

pub trait Loader: Iterator<Item = Event> {}
