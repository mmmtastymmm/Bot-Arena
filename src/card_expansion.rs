use poker::{Card, Suit};

pub trait CardPrinting {
    fn to_ascii_string(&self) -> String;
}

impl CardPrinting for Card {
    fn to_ascii_string(&self) -> String {
        let rank = self.rank().to_string();
        let suit = match self.suit() {
            Suit::Clubs => "C",
            Suit::Hearts => "H",
            Suit::Spades => "S",
            Suit::Diamonds => "D",
        };
        format!("{rank}{suit}")
    }
}

#[cfg(test)]
mod test {
    use poker::{Card, Rank, Suit};

    use crate::card_expansion::CardPrinting;

    #[test]
    fn test_to_string() {
        let test_card = Card::new(Rank::Ace, Suit::Clubs);
        assert_eq!(test_card.to_ascii_string(), "AC");
        let test_card = Card::new(Rank::King, Suit::Hearts);
        assert_eq!(test_card.to_ascii_string(), "KH");
        let test_card = Card::new(Rank::Ten, Suit::Spades);
        assert_eq!(test_card.to_ascii_string(), "TS");
        let test_card = Card::new(Rank::Two, Suit::Diamonds);
        assert_eq!(test_card.to_ascii_string(), "2D");
    }
}
