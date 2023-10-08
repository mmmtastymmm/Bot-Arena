use std::sync::Arc;

use once_cell::sync::Lazy;
use poker::Evaluator;

pub static SHARED_EVALUATOR: Lazy<Arc<Evaluator>> = Lazy::new(|| Arc::new(Evaluator::new()));
