use env_logger::Env;

pub fn enable_logging() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
}

pub fn enable_logging_in_test() {
    let result = env_logger::builder().is_test(true).try_init();
    match result {
        Ok(_) => {}
        Err(_) => {
            println!("Can't setup logger for test!")
        }
    }
}
