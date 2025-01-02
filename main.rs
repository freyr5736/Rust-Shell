use lazy_static::lazy_static; // Importing lazy_static to define static variables initialized at runtime
use std::collections::HashMap; // Importing HashMap for storing commands and their handlers
use std::env; // Importing the env module for interacting with environment variables
use std::io::{self, Write}; // Importing io for reading input and writing output
use std::path::{Path, PathBuf}; // Importing path manipulation functionalities
use std::{fs}; // Importing fs for file system operations
use std::sync::Mutex; // Importing Mutex for safe shared access to mutable data across threads

// Type alias for command handlers: a function that takes a vector of string references and a boxed Write
type CmdHandler = fn(&[&String], Box<dyn Write>);
// A HashMap mapping command names (String) to their handlers (CmdHandler)
type CmdMap = HashMap<String, CmdHandler>;

// Lazy static variable to hold the command map
lazy_static! {
    pub static ref CMD_MAP: CmdMap = {
        let mut cmd_map = CmdMap::new(); // Create a new command map
        cmd_map.insert("cd".to_string(), handle_cd); // Add 'cd' command handler
        cmd_map.insert("pwd".to_string(), handle_pwd); // Add 'pwd' command handler
        cmd_map // Return the populated command map
    };

    // A Mutex-protected variable holding the current working directory
    static ref CURRENT_DIR: Mutex<PathBuf> = Mutex::new(PathBuf::from("/"));
}

// Function to check if a command is a built-in shell command
fn is_builtin(cmd: &str) -> Option<&'static str> {
    match cmd {
        "pwd" => Some("pwd is a shell builtin"), // If command is 'pwd', return a message
        "cd" => Some("cd is a shell builtin"), // If command is 'cd', return a message
        _ => None, // Otherwise, return None
    }
}

// Function to handle the `cd` (change directory) command
fn handle_cd(args: &[&String], mut handle: Box<dyn Write>) {
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")); // Get the home directory

    // Handle tilde expansion for home directory
    let target_dir = if args.is_empty() {
        home_dir.clone() // If no arguments are passed, use the home directory
    } else {
        let mut target = args[0].to_string();

        // Replace tilde (~) with the actual home directory
        if target.starts_with("~") {
            target.replace_range(0..1, &home_dir.to_string_lossy());
        }

        PathBuf::from(target) // Return the expanded target directory as PathBuf
    };

    // Attempt to set the current working directory
    if let Err(e) = env::set_current_dir(&target_dir) { // Set the current directory of the process
        // If an error occurs, output a meaningful error message
        let error_message = e.to_string().split(':').skip(1).collect::<Vec<&str>>().join(":").trim().to_string();
        let error_message = if error_message.is_empty() {
            "No such file or directory".to_string()
        } else {
            error_message
        };
        
        writeln!(handle, "cd: {}: {}", target_dir.display(), error_message).unwrap();
    } else {
        // Update the global `CURRENT_DIR` to reflect the new directory
        let mut current_dir = CURRENT_DIR.lock().unwrap();
        *current_dir = target_dir.clone();

        writeln!(handle, "Changed directory to: {}", target_dir.display()).unwrap();
    }
}

// Function to handle the `pwd` (print working directory) command
fn handle_pwd(_: &[&String], mut handle: Box<dyn Write>) {
    // Access the current directory from the global variable
    let current_dir = CURRENT_DIR.lock().unwrap(); // Mutex lock to safely access the current directory
    writeln!(handle, "{}", current_dir.display()).unwrap(); // Output the current directory
}

// Function to handle a given command
fn handle_cmd(cmd: &str) {
    let args = handle_quotes(cmd); // Handle arguments with quotes
    if args.is_empty() {
        return;
    }

    // Parse redirection (e.g., >, 1>, 2>, >>, etc.)
    let (args, output_file, error_file, append_output, append_error) = parse_redirection(&args);
    let cmd = args[0]; // Extract the command from arguments
    let args = &args[1..]; // Get the remaining arguments

    let mut handle: Box<dyn Write> = Box::new(std::io::stdout()); // Default output handle (stdout)
    let mut stderr_handle: Box<dyn Write> = Box::new(std::io::stderr()); // Default error handle (stderr)

    // Handle output redirection
    if let Some(output_file) = output_file {
        handle = if append_output {
            match fs::OpenOptions::new().append(true).create(true).open(output_file) {
                Ok(file) => Box::new(std::io::BufWriter::new(file)),
                Err(e) => {
                    eprintln!("Failed to open output file {}: {}", output_file, e);
                    return;
                }
            }
        } else {
            match std::fs::File::create(output_file) {
                Ok(file) => Box::new(std::io::BufWriter::new(file)),
                Err(e) => {
                    eprintln!("Failed to create output file {}: {}", output_file, e);
                    return;
                }
            }
        }
    }

    // Handle error redirection
    if let Some(error_file) = error_file {
        stderr_handle = if append_error {
            match fs::OpenOptions::new().append(true).create(true).open(error_file) {
                Ok(file) => Box::new(std::io::BufWriter::new(file)),
                Err(e) => {
                    eprintln!("Failed to open error file {}: {}", error_file, e);
                    return;
                }
            }
        } else {
            match std::fs::File::create(error_file) {
                Ok(file) => Box::new(std::io::BufWriter::new(file)),
                Err(e) => {
                    eprintln!("Failed to create error file {}: {}", error_file, e);
                    return;
                }
            }
        }
    }

    // Check for built-in commands (e.g., 'cd', 'pwd')
    if let Some(builtin_message) = is_builtin(cmd) {
        if cmd != "cd" && cmd != "pwd" { // Print the message for built-in commands other than 'cd' and 'pwd'
            writeln!(handle, "{}", builtin_message).unwrap();
        }
    } else if let Some(builtin_cmd_handler) = CMD_MAP.get(cmd) {
        // If command is in CMD_MAP, execute the corresponding handler
        builtin_cmd_handler(args, handle);
    } else if let Some(path) = check_cmd_in_path(cmd) {
        // For external commands, check if the command exists in the system's PATH
        let cmd_display = path.file_name()
            .map(|os_str| os_str.to_string_lossy().into_owned())
            .unwrap_or_else(|| cmd.to_string());

        // Run the external command with redirection
        let output = std::process::Command::new(cmd)
            .args(args)
            .output();

        match output {
            Ok(output) => {
                handle.write_all(&output.stdout).unwrap();
                stderr_handle.write_all(&output.stderr).unwrap();
            }
            Err(e) => {
                writeln!(handle, "{}: {}", cmd_display, e).unwrap();
            }
        }
    } else {
        eprintln!("{}: command not found", cmd); // If command is not found
    }
}

// Function to check if a command exists in the system's PATH
pub fn check_cmd_in_path(cmd: &str) -> Option<PathBuf> {
    if let Ok(paths) = env::var("PATH") {
        for path in paths.split(':') {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    if cmd == entry.file_name().to_string_lossy() {
                        return Some(entry.path()); // Return the command path if found
                    }
                }
            }
        }
    }
    None // Return None if not found
}

// Function to handle external commands
pub fn handle_path_cmd(cmd: &str, args: &[&String], mut handle: Box<dyn Write>) {
    let cmd_display = Path::new(cmd)
        .file_name()
        .map(|os_str| os_str.to_string_lossy().into_owned())
        .unwrap_or_else(|| cmd.to_string());

    let output = std::process::Command::new(cmd)
        .args(args)
        .output();

    match output {
        Ok(output) => {
            handle.write_all(&output.stdout).unwrap();
            std::io::stderr().write_all(&output.stderr).unwrap();
        }
        Err(e) => {
            writeln!(handle, "{}: {}", cmd_display, e).unwrap();
        }
    }
}

// Function to handle argument parsing and handle quotes properly
fn handle_quotes(args_str: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut arg = String::new();
    let mut inside_single_quotes = false;
    let mut inside_double_quotes = false;
    let mut backslash = false;

    // Loop through each character and handle quotes and escape characters
    for c in args_str.chars() {
        if !inside_single_quotes && !inside_double_quotes {
            if backslash {
                arg.push(c); // Handle escaped characters
                backslash = false;
                continue;
            }
            if c == '\\' {
                backslash = true;
                continue;
            }
            if c == '\'' {
                inside_single_quotes = true;
                continue;
            } else if c == '"' {
                inside_double_quotes = true;
                continue;
            }
            if c.is_whitespace() {
                if !arg.is_empty() {
                    args.push(arg.clone());
                    arg.clear();
                }
                continue;
            }
        } else if inside_single_quotes && c == '\'' {
            inside_single_quotes = false;
            continue;
        } else if inside_double_quotes {
            if backslash {
                backslash = false;
                if c != '$' && c != '"' && c != '\\' {
                    arg.push('\\');
                }
            } else if c == '\\' {
                backslash = true;
                continue;
            } else if c == '"' {
                inside_double_quotes = false;
                continue;
            }
        }
        arg.push(c); // Add character to argument
    }
    if !arg.is_empty() {
        args.push(arg); // Add last argument if not empty
    }
    args
}

// Function to parse redirection operators in the command line (e.g., >, >>, 2>, etc.)
fn parse_redirection<'a>(args: &'a Vec<String>) -> (Vec<&'a String>, Option<&'a String>, Option<&'a String>, bool, bool) {
    let mut cmd_args = Vec::new();
    let mut output_file = None;
    let mut error_file = None;
    let mut append_output = false;
    let mut append_error = false;

    // Iterate through arguments and check for redirection symbols
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            ">" | "1>" => {
                if i + 1 < args.len() {
                    output_file = Some(&args[i + 1]);
                    i += 2;
                    continue;
                }
            }
            "2>" => {
                if i + 1 < args.len() {
                    error_file = Some(&args[i + 1]);
                    i += 2;
                    continue;
                }
            }
            ">>" | "1>>" => {
                if i + 1 < args.len() {
                    output_file = Some(&args[i + 1]);
                    append_output = true;
                    i += 2;
                    continue;
                }
            }
            "2>>" => {
                if i + 1 < args.len() {
                    error_file = Some(&args[i + 1]);
                    append_error = true;
                    i += 2;
                    continue;
                }
            }
            _ => {
                cmd_args.push(&args[i]);
            }
        }
        i += 1;
    }
    (cmd_args, output_file, error_file, append_output, append_error)
}

// Main function
fn main() {
    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        // Print prompt
        print!("$ ");
        io::stdout().flush().unwrap();

        // Read input
        stdin.read_line(&mut input).unwrap();

        // Handle command
        let cmd = input.trim();
        if cmd.is_empty() {
            input.clear();
            continue;
        }
        handle_cmd(cmd); // Handle the entered command
        input.clear();
    }
}

