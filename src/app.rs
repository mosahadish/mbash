use anyhow::{Context, Result, anyhow};
use logger::{Logger, debug, error, info};
use std::{
    fs,
    process::{Command, ExitCode, exit},
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

const TRACKING_FILE_NAME: &str = ".mtracking";
const IGNORE_FILE_NAME: &str = ".mignoring";

pub struct Mbash {
    exiting: Arc<AtomicBool>,
    current_path: PathBuf,
    tracking_files: Vec<String>,
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
            tracking_files: Vec::new(),
            exit_command: "exit",
        }
    }

    pub fn setup(&mut self) -> Result<()> {
        self.set_current_dir();
        self.load_tracking_file()
            .context("Failed to setup mbash, failed to load tracking file.")?;

        Ok(())
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

                    self.handle_input(command_line);
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

    fn handle_input(&mut self, input_line: &str) {
        debug!(self.logger, "Received input '{}'.", input_line);

        let parts: Vec<&str> = input_line.split_whitespace().collect();
        if parts.is_empty() {
            debug!(
                self.logger,
                "Splitting the input using whitespaces resulted in an empty vector."
            );
            return;
        }

        let first_word = parts[0];
        if first_word == self.internal_command_prefix {
            debug!(self.logger, "Received an internal command.");
            let command_name = parts[1];
            let args = &parts[2..];

            self.execute_internal_command(command_name, args);
            return;
        }

        let command_name = parts[0];
        let args = &parts[1..];

        if command_name == self.exit_command {
            self.exit();
            info!(
                self.logger,
                "Received '{}' command, exiting mbash.", self.exit_command
            );

            self.exit();
            return;
        }

        self.execute_external_command(command_name, args);
    }

    fn execute_internal_command(&mut self, command_name: &str, args: &[&str]) {
        debug!(self.logger, "Received internal '{}' command.", command_name);
        if command_name == "init" {
            // TODO
            _ = helper_functions::attempt_create_file(IGNORE_FILE_NAME);
            _ = helper_functions::attempt_create_file(TRACKING_FILE_NAME);
            return;
        }
        if command_name == "cd" {
            self.cd(args);
        }
    }

    fn execute_external_command(&mut self, command_name: &str, args: &[&str]) {
        debug!(self.logger, "Received external '{}' command.", command_name);

        if command_name == "cd" {
            self.cd(args);
            return;
        }
        if command_name == "ls" {
            self.list_files(args)
        }
    }

    fn list_files(&mut self, args: &[&str]) {
        match std::fs::read_dir(&self.current_path) {
            Ok(entries) => {
                for entry_result in entries {
                    match entry_result {
                        Ok(entry) => {
                            let file_name = entry.file_name();
                            let file_type_result = entry.file_type();
                            match file_type_result {
                                Ok(file_type) => {
                                    let is_dir = file_type.is_dir();
                                    if is_dir {
                                        println!("{} [DIR]", file_name.to_string_lossy(),);
                                    }
                                }
                                Err(e) => {
                                    debug!(self.logger, "Failed determining whether '{}' is a dir or not due to an error '{}'", file_name.to_string_lossy(), e);
                                    println!("{} [?]", file_name.to_string_lossy());
                                }
                            }
                        }
                        Err(e) => {
                            error!(self.logger, "Error reading file entry: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                error!(
                    self.logger,
                    "Failed to read current directory's contents due to an error '{}'. 'ls' command failed.",
                    e
                );
            }
        }
    }

    fn cd(&mut self, args: &[&str]) {
        debug!(self.logger, "huh");
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

        self.exiting.store(true, Ordering::Relaxed);
    }

    fn load_tracking_file(&mut self) -> io::Result<()> {
        let _ = helper_functions::attempt_create_file(TRACKING_FILE_NAME)?;
        let read_result = fs::read_to_string(TRACKING_FILE_NAME);
        match read_result {
            Ok(file_contents) => {
                if file_contents.is_empty() {
                    debug!(self.logger, "Currently not tracking anything.");
                    return Ok(());
                }

                let parts = file_contents.split("\n");
                for part in parts {
                    debug!(self.logger, "Tracking '{}'", part);
                    self.tracking_files.push(part.to_string());
                }

                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}
