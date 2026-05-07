use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Text(String),
    Boolean(bool),
    Tuple(Vec<Value>),
    Map(Vec<(String, Value)>),
    Opaque(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Text(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Tuple(values) => {
                write!(f, "(")?;
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, ")")
            }
            Value::Map(entries) => {
                write!(f, "{{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Opaque(s) => write!(f, "<{}>", s),
        }
    }
}

impl Value {
    pub fn number(n: f64) -> Self {
        Value::Number(n)
    }

    pub fn text<S: Into<String>>(s: S) -> Self {
        Value::Text(s.into())
    }

    pub fn boolean(b: bool) -> Self {
        Value::Boolean(b)
    }

    pub fn tuple(values: Vec<Value>) -> Self {
        Value::Tuple(values)
    }

    pub fn map(entries: Vec<(String, Value)>) -> Self {
        Value::Map(entries)
    }

    pub fn opaque<S: Into<String>>(s: S) -> Self {
        Value::Opaque(s.into())
    }
}
