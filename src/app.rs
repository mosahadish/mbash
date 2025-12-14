use logger::{Logger, debug, error, info};
use std::process::{Command};

use crate::helper_functions;
use std::{
    env,
    io::{self, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

const TRACKING_FILE_PATH: &str = ".mtracking";
const IGNORE_FILE_PATH: &str = ".mignoring";

pub struct Mbash {
    exiting: Arc<AtomicBool>,
    current_path: PathBuf,
    logger: Box<dyn Logger>,
    internal_command_prefix: &'static str,
    exit_command: &'static str,
}

impl Mbash {
    pub fn new(logger: Box<dyn Logger>) -> Self {
        Mbash {
            exiting: Arc::new(AtomicBool::new(false)),
            current_path: PathBuf::new(),
            logger: logger,
            internal_command_prefix: "m",
            exit_command: "exit",
        }
    }

    pub fn setup(&mut self) {
        self.set_current_dir();
        self.load_file(TRACKING_FILE_PATH);
        self.load_file(IGNORE_FILE_PATH);
    }

    fn set_current_dir(&mut self) {
        let current_dir_result = env::current_dir();
        match current_dir_result {
            Ok(path) => self.current_path = path,
            Err(e) => {
                error!(self.logger, "Failed to fetch current directory path. {}", e);
                self.exit();
            }
        }
    }

    pub fn run(&mut self) {
        while !self.exiting.load(Ordering::Relaxed) {
            print!("mbash@ {}: ", self.current_path.display());

            let flush_result = io::stdout().flush();
            match flush_result {
                Ok(_) => (),
                Err(e) => {
                    error!(self.logger, "Flush failed due to an error {}", e);
                    continue;
                }
            }

            let mut input = String::new();

            let read_result = io::stdin().read_line(&mut input);
            match read_result {
                Ok(_) => {
                    let command_line = input.trim();
                    if command_line.is_empty() {
                        debug!(self.logger, "User input is empty.");
                        continue;
                    }

                    let parts: Vec<&str> = command_line.split_whitespace().collect();
                    if parts.is_empty() {
                        debug!(
                            self.logger,
                            "Splitting the input using whitespaces resulted in an empty vector."
                        );
                        continue;
                    }

                    let first_word = parts[0];
                    if first_word == self.internal_command_prefix {
                        debug!(self.logger, "Received an internal command.");
                        continue;
                    }

                    let command_name = parts[0];
                    let args = &parts[1..];

                    if command_name == self.exit_command {
                        self.exit();
                        info!(
                            self.logger,
                            "Received '{}' command, exiting mbash.", self.exit_command
                        );
                        break;
                    }

                    self.execute_external_command(command_name, args);
                }
                Err(e) => {
                    error!(
                        self.logger,
                        "Failed to read user input due to an error '{}'.", e
                    );

                    continue;
                }
            }
        }
    }

    fn execute_external_command(&mut self, command_name: &str, args: &[&str]) {
        if command_name.starts_with("cd") {
            self.handle_cd_command(args);
            return;
        }

        debug!(self.logger, "{}", command_name);
        for arg in args {
            debug!(self.logger, "{}", arg);
        }

        let mut command = Command::new(command_name);
        command.args(args);

        match command.status() {
            Ok(status) => {
                if !status.success() {
                    error!(
                        self.logger,
                        "Command '{}' failed with status: {}", command_name, status
                    );
                    return;
                }

                debug!(
                    self.logger,
                    "Command '{} suceeeded with status '{}'.", command_name, status
                );
            }
            Err(e) => {
                error!(
                    self.logger,
                    "Failed to execute command '{}': {}", command_name, e
                );
            }
        }
    }

    fn handle_cd_command(&mut self, args: &[&str]) {
        let new_dir = &args[0];

        if new_dir.is_empty() {
            debug!(
                self.logger,
                "'cd' command requires a directory as an argument [cd <directory>]."
            );
            return;
        }

        match env::set_current_dir(new_dir) {
            Ok(()) => {
                debug!(self.logger, "Changed directory to '{}'.", new_dir);
                self.set_current_dir();
            }
            Err(e) => {
                error!(
                    self.logger,
                    "Failed to change directory to '{}': '{}'.", new_dir, e
                );
            }
        }
    }

    pub fn exit(&self) {
        if self.exiting.load(Ordering::Relaxed) {
            return;
        }

        self.exiting.load(Ordering::Relaxed);
    }

    fn load_file(&self, file_name: &str) {
        helper_functions::attempt_create_file(file_name);
        // todo
    }
}
