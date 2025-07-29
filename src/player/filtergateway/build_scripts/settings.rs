// Module for loading DDS settings
use serde_yaml;
use std::fs;
use std::path::PathBuf;

/// Load DDS settings from settings.yaml (during build)
pub fn load_dds_settings() -> Result<(PathBuf, i32, Option<String>), Box<dyn std::error::Error>> {
    // Set default values (used when no settings file exists)
    // Path relative to project root
    let default_idl_dir = PathBuf::from("src/vehicle/dds/idl");
    let default_domain_id = 0;
    let default_out_dir = None; // Default is to use the OUT_DIR environment variable

    // Search for settings file path based on project root
    // CARGO_MANIFEST_DIR is the path where filtergateway's Cargo.toml is located
    // To change to a relative path based on pullpiri (project root), move to parent directory
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let settings_path = PathBuf::from(&manifest_dir)
        .parent() // go up to 'player'
        .and_then(|p| p.parent()) // go up to 'src'
        .and_then(|p| p.parent()) // go up to 'pullpiri'
        .map(|p| p.join("src/settings.yaml"))
        .ok_or("Failed to resolve project root for settings.yaml")?;

    let mut settings_content = String::new();

    if !settings_path.exists() {
        println!("No settings file found, using defaults");
        return Ok((default_idl_dir, default_domain_id, default_out_dir));
    }

    // Read settings file
    println!("Reading settings from: {:?}", settings_path);
    settings_content = fs::read_to_string(&settings_path)?;

    // Parse JSON or YAML
    let settings: serde_yaml::Value = serde_yaml::from_str(&settings_content)?;

    println!("Settings content: {}", settings_content);

    // Extract DDS settings (relative path based on project root)
    let idl_path = settings
        .get("dds")
        .and_then(|dds| dds.get("idl_path"))
        .and_then(|path| path.as_str())
        .map(PathBuf::from)
        .unwrap_or(default_idl_dir);

    // Calculate absolute path based on project root
    println!("IDL path from settings (relative): {:?}", idl_path);

    let domain_id = settings
        .get("dds")
        .and_then(|dds| dds.get("domain_id"))
        .and_then(|id| id.as_i64())
        .map(|id| id as i32)
        .unwrap_or(default_domain_id);

    println!("Domain ID from settings: {}", domain_id);

    // Check for custom OUT_DIR value
    // If relative, convert to absolute path within target directory
    let out_dir = settings
        .get("dds")
        .and_then(|dds| dds.get("out_dir"))
        .and_then(|path| path.as_str())
        .and_then(|dir| {
            // If directory starts with / consider it absolute, otherwise make it relative to build directory
            if dir.starts_with('/') {
                Some(dir.to_string())
            } else {
                // Use the standard cargo OUT_DIR as the base directory
                std::env::var("OUT_DIR").ok().map(|out_dir| {
                    println!(
                        "Converting relative path '{}' to absolute within build directory",
                        dir
                    );
                    format!("{}", out_dir)
                })
            }
        });

    if let Some(dir) = &out_dir {
        println!("Output directory (absolute): {}", dir);
    }

    Ok((idl_path, domain_id, out_dir))
}
