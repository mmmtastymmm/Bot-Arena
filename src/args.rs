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
