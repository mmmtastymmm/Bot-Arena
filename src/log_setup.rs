/// Allow log macros to be captured by the test that calls this function
#[cfg(test)]
pub fn enable_logging_in_test() {
    let _ = env_logger::builder()
        .filter(None, log::LevelFilter::Info)
        .is_test(true)
        .try_init();
}
