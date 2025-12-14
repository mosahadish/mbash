use logger::{Logger, debug, error, stdout_logger::StdoutLogger};
use std::{
    env::set_current_dir,
    process::{Command, exit},
};

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
}

impl Mbash {
    pub fn new(logger: Box<dyn Logger>) -> Self {
        Mbash {
            exiting: Arc::new(AtomicBool::new(false)),
            current_path: PathBuf::new(),
            logger: logger,
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
            io::stdout().flush().expect("Failed to flush stdout");

            let mut input = String::new();

            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");

            let command_line = input.trim();
            if command_line == "exit" {
                self.exit();
                break;
            }

            if command_line.starts_with("cd") {
                self.handle_cd_command(command_line);
                continue;
            }
            if command_line.is_empty() {
                continue; 
            }

            self.execute_external_command(command_line);
        }
    }

    fn execute_external_command(&self, command_line: &str) {
        let parts: Vec<&str> = command_line.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        let command_name = parts[0];
        let args = &parts[1..];

        let mut command = Command::new(command_name);
        command.args(args);

        match command.status() {
            Ok(status) => {
                if !status.success() {
                    error!(self.logger, "Command '{}' failed with status: {}", command_name, status);
                }
            }
            Err(e) => {
                error!(self.logger, "Failed to execute command '{}': {}", command_name, e);
            }
        }
    }

    fn handle_cd_command(&mut self, command_line: &str) {
        let new_dir = &command_line[3..].trim();

        if new_dir.is_empty() {
            debug!(self.logger, "'cd' command requires a directory as an argument [cd <directory>].");
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
