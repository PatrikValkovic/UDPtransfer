use udp_transfer::receiver::{logic, config::Config};

fn main() {
    let config = Config::from_command_line();
    let is_verbose = config.is_verbose();

    if let Err(e) = logic::logic(config) {
        println!("Ending program because of error");
        if is_verbose {
            println!("{}", e);
        }
    }
}