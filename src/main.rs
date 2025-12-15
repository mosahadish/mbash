mod app;
mod helper_functions;

use app::Mbash;
use logger::{LogLevel, Logger, error, stdout_logger::StdoutLogger};

fn main() {
    let logger: Box<dyn Logger> = Box::new(StdoutLogger::new(LogLevel::DEBUG));
    let mut mbash = Mbash::new(logger);
    match mbash.setup() {
        Ok(_) => {
            mbash.run();
        },
        Err(e) => eprintln!("Mbash setup process failed. Check logs for more details. '{}'.", e)
    }
}
