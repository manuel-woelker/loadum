use crate::LoadumString;

pub enum Value {
    Null,
    Boolean(bool),
    Number(f64),
    String(LoadumString),
}

impl Value {
    pub fn string(value: impl Into<LoadumString>) -> Value {
        Value::String(value.into()) //value.into()) //Value::String(value.into())
    }
    pub fn number(value: impl Into<f64>) -> Value {
        Value::Number(value.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::value::Value;
    use ecow::EcoString;

    #[test]
    fn value_size() {
        assert_eq!(size_of::<Value>(), 24);
        assert_eq!(size_of::<Box<String>>(), 8);
        assert_eq!(size_of::<String>(), 24);
        assert_eq!(size_of::<EcoString>(), 16);
    }
}
