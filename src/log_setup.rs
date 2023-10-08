/// Allow log macros are captured and printed by the test if this is called.
#[cfg(test)]
pub fn enable_logging_in_test() {
    let _ = env_logger::builder()
        .filter(None, log::LevelFilter::Info)
        .is_test(true)
        .try_init();
}
