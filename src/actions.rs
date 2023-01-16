use std::fmt;
use std::fmt::Formatter;

enum Actions {
    Fold,
    Call,
    Raise(i32),
    Check,
}

impl fmt::Display for Actions {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Actions::Fold => { write!(f, "Fold") }
            Actions::Call => { write!(f, "Call") }
            Actions::Raise(a) => { write!(f, "Raise: {}", a) }
            Actions::Check => { write!(f, "Check") }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    pub fn test_print() {
        assert_eq!(Actions::Call.to_string(), "Call");
        assert_eq!(Actions::Fold.to_string(), "Fold");
        assert_eq!(Actions::Check.to_string(), "Check");
        assert_eq!(Actions::Raise(23).to_string(), "Raise: 23");
    }
}