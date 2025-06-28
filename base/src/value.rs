pub enum Value {
    Null,
    Boolean(bool),
    Number(f64),
    String(Box<String>),
}

impl Value {
    pub fn string(value: impl Into<String>) -> Value {
        Value::String(Box::new(value.into()))
    }
    pub fn number(value: impl Into<f64>) -> Value {
        Value::Number(value.into())
    }
}
