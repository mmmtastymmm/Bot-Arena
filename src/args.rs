use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct BotArgs {
    /// The port the server to listen on
    #[arg(short, long, default_value_t = 10100)]
    pub port: i32,

    /// How long for the server to wait to accept connections
    #[arg(short, long, default_value_t = 30.0)]
    pub server_connection_time_seconds: f64,

    /// Number of bots that only call (for testing)
    #[arg(short = 'c', long, default_value_t = 0)]
    pub n_call_bots: usize,

    /// Number of bots that take random actions (for testing)
    #[arg(short = 'r', long, default_value_t = 0)]
    pub n_random_bots: usize,

    /// Number of bots that only fold (for testing)
    #[arg(short = 'f', long, default_value_t = 0)]
    pub n_fail_bots: usize,

    /// Override to disable all logging if passed
    #[arg(short = 'l', long)]
    pub disable_logging: bool,
}

// Validation function to ensure the sum of call-bot and random-bot is less than 23
pub fn validate_bot_args(args: &BotArgs) -> Result<(), String> {
    let sum = args.n_call_bots + args.n_random_bots + args.n_fail_bots;
    if sum >= 23 {
        Err("The sum of all bots must be less than 23".to_string())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::args::{validate_bot_args, BotArgs};

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
    fn test_disable_logging_arg() {
        let args = BotArgs::parse_from(vec!["test"]);
        assert!(!args.disable_logging);
        let args = BotArgs::parse_from(vec!["test", "--disable-logging"]);
        assert!(args.disable_logging);
        let args = BotArgs::parse_from(vec!["test", "-l"]);
        assert!(args.disable_logging);
    }

    #[test]
    fn test_custom_connection_time() {
        let args = BotArgs::parse_from(vec!["test", "--server-connection-time-seconds", "45.0"]);
        assert_eq!(args.server_connection_time_seconds, 45.0);
    }

    #[test]
    fn test_bot_args_sum() {
        let args = BotArgs::parse_from(vec![
            "test",
            "--n-call-bots",
            "10",
            "--n-random-bots",
            "10",
            "--n-fail-bots",
            "3",
        ]);

        // The sum should be valid and less than 23
        assert!(validate_bot_args(&args).is_err());
    }
}
