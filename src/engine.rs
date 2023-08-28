use std::sync::Arc;

use poker::Evaluator;

use crate::table::Table;

pub struct Engine {
    pub table: Table,
}

impl Engine {
    pub fn new(number_of_players: usize, evaluator: Arc<Evaluator>) -> Engine {
        Engine {
            table: Table::new(number_of_players, evaluator),
        }
    }
}
