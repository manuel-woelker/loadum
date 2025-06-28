use crate::LoadumString;
use crate::value::Value;

pub enum Event {
    DocumentStart,
    DocumentEnd,
    MapStart,
    MapEnd,
    ListStart,
    ListEnd,
    MapKey(Value),
    Literal(Value),
}

impl Event {
    pub fn null() -> Event {
        Event::Literal(Value::Null)
    }
    pub fn bool(value: bool) -> Event {
        Event::Literal(Value::Boolean(value))
    }

    pub fn string(s: impl Into<LoadumString>) -> Event {
        Event::Literal(Value::string(s))
    }
    pub fn number(value: impl Into<f64>) -> Event {
        Event::Literal(Value::number(value))
    }

    pub fn map_key(s: impl Into<LoadumString>) -> Event {
        Event::MapKey(Value::string(s))
    }
}

#[cfg(test)]
mod tests {
    use crate::event::Event;

    #[test]
    fn event_size() {
        assert_eq!(size_of::<Event>(), 32);
    }
}
