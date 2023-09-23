use env_logger::Env;

/// Call once to enable logging for the program
pub fn enable_logging() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
}

#[test]
#[should_panic]
pub fn test_enable_logging() {
    // Multiple inits should fail
    enable_logging();
    enable_logging();
}

/// Allow log macros to be captured by the test that calls this function
#[test]
pub fn enable_logging_in_test() {
    let result = env_logger::builder().is_test(true).try_init();
    match result {
        Ok(_) => {}
        Err(_) => {
            println!("Can't setup logger for test!")
        }
    }
}
