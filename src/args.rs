use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct BotArgs {
    // The port to listen to
    #[arg(short, long, default_value_t = 10100)]
    pub port: i32,
    // How long to wait to accept connections
    #[arg(short, long, default_value_t = 30.0)]
    pub server_connection_time_seconds: f64,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::args::BotArgs;

    #[test]
    fn test_defaults() {
        let args = BotArgs::parse_from(vec!["test"]);
        assert_eq!(args.port, 10100);
        assert_eq!(args.server_connection_time_seconds, 30.0);
    }

    #[test]
    fn test_custom_port() {
        let args = BotArgs::parse_from(vec!["test", "--port", "8080"]);
        assert_eq!(args.port, 8080);
    }

    #[test]
    fn test_custom_connection_time() {
        let args = BotArgs::parse_from(vec!["test", "--server-connection-time-seconds", "45.0"]);
        assert_eq!(args.server_connection_time_seconds, 45.0);
    }
}
