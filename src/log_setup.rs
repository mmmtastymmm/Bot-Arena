/// Allow log macros to be captured by the test that calls this function
#[test]
pub fn enable_logging_in_test() {
    let _ = env_logger::builder().is_test(true).try_init();
}
