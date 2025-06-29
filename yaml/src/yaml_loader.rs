use loadum::event::Event;
use loadum::loader::Loader;

pub struct YamlLoader<'source> {
    _source: &'source str,
}

impl YamlLoader<'_> {
    pub fn new(source: &str) -> YamlLoader {
        YamlLoader { _source: source }
    }
}

impl<'source> Loader for YamlLoader<'source> {}

impl<'source> Iterator for YamlLoader<'source> {
    type Item = Event;
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
