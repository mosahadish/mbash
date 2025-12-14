mod app;
mod helper_functions;

use app::Mbash;
use logger::{LogLevel, Logger, stdout_logger::StdoutLogger};

fn main() {
    let logger: Box<dyn Logger> = Box::new(StdoutLogger::new(LogLevel::DEBUG));
    let mut mbash = Mbash::new(logger);
    mbash.setup();

    let _ = mbash.run();
}