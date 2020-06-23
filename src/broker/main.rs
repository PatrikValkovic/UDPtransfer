mod packet_wrapper;
mod config;
mod broker;

fn main() {
    let config = config::Config::from_command_line();

    broker::broker(config);
}