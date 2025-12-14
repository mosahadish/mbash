use logger::LogLevel;
use logger::Logger;
use logger::debug;
use logger::error;
use logger::stdout_logger::StdoutLogger;
use std::fs::File;

/// Creates a new file as long as it doesn't exist
pub fn attempt_create_file(file_name: &str) -> bool {
    let logger: Box<dyn Logger> = Box::new(StdoutLogger::new(LogLevel::DEBUG));

    match std::fs::exists(file_name) {
        Ok(true) => {
            debug!(logger, "'{}' already exists.", file_name);
            return true;
        }
        Ok(false) => {
            debug!(logger, "'{}' doesn't exist!", file_name);
            debug!(logger, "Attempting to create '{}' file", file_name);

            let file_creation_result = File::create(file_name);
            match file_creation_result {
                Ok(_) => debug!(logger, "Successfully created '{}' file!", file_name),
                Err(e) => error!(logger, "Failed to create '{}' file! {}", file_name, e),
            };

            return true;
        }

        Err(e) => {
            error!(
                logger,
                "An error occured while trying to determine whether '{}' file exists or not. {}",
                file_name,
                e
            );
            return false;
        }
    }
}
