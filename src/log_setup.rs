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
