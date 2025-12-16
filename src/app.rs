use anyhow::{Context, Result, anyhow};
use logger::{Logger, debug, error, info};
use std::{
    collections::HashMap,
    fs,
    process::{Command, ExitCode},
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
    commands: HashMap<String, fn(&mut Mbash, &[&str])>,
}

impl Mbash {
    pub fn new(logger: Box<dyn Logger>) -> Self {
        let mut command_map: HashMap<String, fn(&mut Self, &[&str])> = HashMap::new();

        command_map.insert("ls".to_string(), list_files);
        command_map.insert("cd".to_string(), cd);
        command_map.insert("init".to_string(), init);
        command_map.insert("exit".to_string(), exit);

        Mbash {
            exiting: Arc::new(AtomicBool::new(false)),
            current_path: PathBuf::new(),
            logger: logger,
            internal_command_prefix: "m",
            tracking_files: Vec::new(),
            commands: command_map,
        }
    }

    pub fn setup(&mut self) -> Result<()> {
        self.set_current_dir()
            .context("Failed to setup mbash, failed to set current dir.")?;
        self.load_tracking_file()
            .context("Failed to setup mbash, failed to load tracking file.")?;

        Ok(())
    }

    fn set_current_dir(&mut self) -> Result<()> {
        let current_dir_result = env::current_dir();
        match current_dir_result {
            Ok(path) => {
                self.current_path = path;
                Ok(())
            }
            Err(e) => {
                error!(self.logger, "Failed to fetch current directory path. {}", e);
                Err(e.into())
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
        let mut command_name_index = 0;
        if first_word == self.internal_command_prefix {
            command_name_index += 1;
        }

        let args_index = command_name_index + 1;

        let command_name = parts[command_name_index];
        let args = &parts[args_index..];

        if self.commands.contains_key(command_name) {
            self.commands[command_name](self, args);
        }
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

fn list_files(mbash: &mut Mbash, args: &[&str]) {
    match std::fs::read_dir(&mbash.current_path) {
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
                                } else {
                                    println!("{}", file_name.to_string_lossy(),);
                                }
                            }
                            Err(e) => {
                                debug!(
                                    mbash.logger,
                                    "Failed determining whether '{}' is a dir or not due to an error '{}'",
                                    file_name.to_string_lossy(),
                                    e
                                );
                                println!("{} [?]", file_name.to_string_lossy());
                            }
                        }
                    }
                    Err(e) => {
                        error!(mbash.logger, "Error reading file entry: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            error!(
                mbash.logger,
                "Failed to read current directory's contents due to an error '{}'. 'ls' command failed.",
                e
            );
        }
    }
}

fn cd(mbash: &mut Mbash, args: &[&str]) {
    let new_dir = &args[0];
    if new_dir.is_empty() {
        debug!(
            mbash.logger,
            "'cd' command requires a directory as an argument [cd <directory>]."
        );
        return;
    }

    match env::set_current_dir(new_dir) {
        Ok(()) => {
            debug!(mbash.logger, "Changed directory to '{}'.", new_dir);
            mbash.set_current_dir();
        }
        Err(e) => {
            error!(
                mbash.logger,
                "Failed to change directory to '{}': '{}'.", new_dir, e
            );
        }
    }
}

fn init(mbash: &mut Mbash, args: &[&str]) {
    // TODO
    _ = helper_functions::attempt_create_file(IGNORE_FILE_NAME);
    _ = helper_functions::attempt_create_file(TRACKING_FILE_NAME);
}

fn exit(mbash: &mut Mbash, args: &[&str]) {
    if mbash.exiting.load(Ordering::Relaxed) {
        return;
    }

    mbash.exiting.store(true, Ordering::Relaxed);
}
