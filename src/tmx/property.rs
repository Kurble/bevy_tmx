use std::path::PathBuf;

/// A custom property
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Property {
    String(String),
    Int(i32),
    Float(f64),
    Bool(bool),
    /// A color in the format `[a, r, g, b]`
    Color([u8; 4]),
    File(String),
}

impl Property {
    /// Return &str value if this property is a string, `None` otherwise.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Property::String(x) => Some(x.as_str()),
            _ => None,
        }
    }

    /// Return i32 value if this property is an int or float, `None` otherwise.
    pub fn as_int(&self) -> Option<i32> {
        match *self {
            Property::Int(x) => Some(x),
            Property::Float(x) => Some(x as i32),
            _ => None,
        }
    }

    /// Return f64 value if this property is a float or int, `None` otherwise.
    pub fn as_float(&self) -> Option<f64> {
        match *self {
            Property::Float(x) => Some(x),
            Property::Int(x) => Some(x as f64),
            _ => None,
        }
    }

    /// Return bool value if this property is a bool, `None` otherwise.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Property::Bool(x) => Some(x),
            _ => None,
        }
    }

    /// Return `[u8; 4]` value if this property is a color, `None` otherwise.
    pub fn as_color(&self) -> Option<[u8; 4]> {
        match *self {
            Property::Color(x) => Some(x),
            _ => None,
        }
    }

    /// Return PathBuf value if this property is a file, `None` otherwise.
    pub fn as_file(&self) -> Option<PathBuf> {
        match self {
            Property::File(x) => Some(PathBuf::from(x)),
            _ => None,
        }
    }
}
