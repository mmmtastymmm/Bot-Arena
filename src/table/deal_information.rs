use std::fmt;
use std::fmt::Formatter;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct DealInformation {
    pub round_number: i32,
    pub dealer_button_index: usize,
}

impl fmt::Display for DealInformation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "hand number: {}, dealer index: {}",
            self.round_number, self.dealer_button_index
        )
    }
}
