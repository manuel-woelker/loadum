use crate::event::Event;
use crate::result::LoadumResult;

pub trait Dumper {
    fn emit(&mut self, event: &Event) -> LoadumResult<()>;
}
