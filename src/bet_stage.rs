use crate::bet_stage::BetStage::{Flop, PreFlop, River, Turn};

#[derive(Eq, PartialEq)]
pub enum BetStage {
    PreFlop,
    Flop,
    Turn,
    River,
}

impl BetStage {
    pub fn next_stage(&mut self) {
        match self {
            PreFlop => { *self = Flop; }
            Flop => { *self = Turn; }
            Turn => { *self = River }
            River => { *self = PreFlop; }
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::bet_stage::BetStage;

    #[test]
    fn check_next_stage() {
        let mut bet_stage = BetStage::PreFlop;
        bet_stage.next_stage();
        assert!(bet_stage == BetStage::Flop);
        bet_stage.next_stage();
        assert!(bet_stage == BetStage::Turn);
        bet_stage.next_stage();
        assert!(bet_stage == BetStage::River);
        bet_stage.next_stage();
        assert!(bet_stage == BetStage::PreFlop);
    }
}