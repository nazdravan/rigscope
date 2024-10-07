use serde::{Deserialize, Serialize};

/// A value that may be known, unknown, gated behind root, or unavailable for a stated reason.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Field<T> {
    Known(T),
    #[default]
    Unknown,
    NeedsRoot,
    Unavailable(String),
}

impl<T> Field<T> {
    /// The value, if known.
    pub fn known(&self) -> Option<&T> {
        match self {
            Field::Known(v) => Some(v),
            _ => None,
        }
    }

    /// `Some(v)` → `Known(v)`, `None` → `Unknown`.
    pub fn from_option(o: Option<T>) -> Self {
        o.map_or(Field::Unknown, Field::Known)
    }
}
