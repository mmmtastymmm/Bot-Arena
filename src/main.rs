use poker::{Card, Rank};

fn get_deck() -> Vec<Card> {
    let deck = Card::generate_deck().collect();
    deck
}


fn main() {
    println!("Hello, world!");
    let deck = get_deck();
    println!("Have this many cards: {}", deck.len());
}

#[test]
fn check_deck_size() {
    let deck: Vec<Card> = get_deck();
    assert_eq!(deck.len(), 52);
    let example_rank = deck.get(0).unwrap().rank();
    // Check the rank is reasonable
    assert!(Rank::Ace >= example_rank);
    assert!(Rank::Two <= example_rank);
}

#[test]
fn check_main() {
    main()
}