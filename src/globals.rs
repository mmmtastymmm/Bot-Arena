use std::sync::Arc;

use once_cell::sync::Lazy;
use poker::Evaluator;

// A static Evaluator, since only one is needed and can then be shared.
pub static SHARED_EVALUATOR: Lazy<Arc<Evaluator>> = Lazy::new(|| Arc::new(Evaluator::new()));
