use udp_transfer::broker::{config::Config, logic};

fn main() {
    let config = Config::from_command_line();

    logic::broker(config);
}