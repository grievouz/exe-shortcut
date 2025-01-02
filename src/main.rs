use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Child};
use shared_child::SharedChild;
use serde::Deserialize;
use anyhow::{Context, Result};
use std::sync::Arc;

#[derive(Clone, Deserialize)]
struct LaunchConfig {
    target: String,
    start_in: Option<String>,
    arguments: Option<Vec<String>>,
}

fn launch_program(config_path: &Path) -> Result<SharedChild> {
    let config_content = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    let config: LaunchConfig = toml::from_str(&config_content)
        .context("Failed to parse TOML config")?;

    // Build the command
    let mut command = Command::new(&config.target);

    // Set working directory if specified
    if let Some(start_in) = config.start_in {
        command.current_dir(start_in);
    }

    // Add arguments if specified
    if let Some(args) = config.arguments {
        command.args(args);
    }

    // Launch the program
    println!("Launching: {}", config.target);
    SharedChild::spawn(&mut command)
        .with_context(|| format!("Failed to launch program: {}", config.target))
}

fn main() -> Result<()> {
    let config_path = get_config_path();

    if !config_path.exists() {
        eprintln!("Config file not found: {}", config_path.display());
        eprintln!("Usage: {} [path-to-config.toml]", env::current_exe()?.display());
        eprintln!("If no path is provided, program will look for a .toml file with the same name as the executable.");
        std::process::exit(1);
    }

    let shared_child = launch_program(&config_path)?;
    let child_arc = Arc::new(shared_child);

    let child_handle = child_arc.clone();
    ctrlc::set_handler(move || {
        println!("\nReceived termination signal, forwarding to child process...");
        child_handle.kill().context("Failed to wait for child process").unwrap()
    }).expect("Error setting Ctrl-C handler");

    let status = child_arc.wait()?;
    println!("Target process exited with status: {:?}", status);
    Ok(())

}

fn get_config_path() -> PathBuf {
    let exe_path = env::current_exe().expect("Failed to get executable path");
    let mut config_path = exe_path.with_extension("toml");

    // If command line argument is provided, use that instead
    if let Some(arg) = env::args().nth(1) {
        config_path = PathBuf::from(arg);
    }

    config_path
}
