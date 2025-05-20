// Refactored build.rs file - Modular structure
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;

// Include modules
mod build_scripts;

use build_scripts::generator::{
    create_empty_modules, generate_dds_module, generate_type_metadata_registry, generate_type_registry
};
use build_scripts::idl::collect_idl_files;
use build_scripts::settings::load_dds_settings;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/vehicle/dds/idl");
    println!("cargo:rerun-if-changed=/home/edo/2025/projects/pullpiri/src/settings.yaml");

    // Check current working directory (project root)
    let current_dir = env::current_dir().expect("Cannot determine current directory");
    println!("Current working directory: {:?}", current_dir);

    // Load settings
    let (rel_idl_dir, domain_id, custom_out_dir) = load_dds_settings()?;

    // Convert relative path to absolute path based on current working directory
    let idl_dir = current_dir.join(&rel_idl_dir);
    println!("IDL directory (absolute): {:?}", idl_dir);
    println!("Domain ID: {}", domain_id);

    // Check IDL directory (do not create if not exists)
    if !idl_dir.exists() {
        println!("Warning: IDL directory doesn't exist: {:?}", idl_dir);
        println!("No files will be processed");
    } else {
        // Output absolute path (for debugging)
        if let Ok(abs_path) = idl_dir.canonicalize() {
            println!("IDL directory absolute path: {:?}", abs_path);
        }
    }

    // Configure output directory (use the one specified in settings.yaml if provided)
    let out_dir = match custom_out_dir {
        Some(dir) => {
            // Use custom directory if specified
            let path = PathBuf::from(&dir);
            println!("Using custom output directory: {}", path.display());

            // Create directory if it doesn't exist
            if !path.exists() {
                println!("Creating custom output directory");
                fs::create_dir_all(&path)?;
            }

            // Set environment variable OUT_DIR (so other build scripts can use it too)
            println!("cargo:rustc-env=OUT_DIR={}", path.display());

            dir
        }
        None => {
            // Use default OUT_DIR environment variable
            let dir = env::var("OUT_DIR").expect("OUT_DIR environment variable not found");
            println!("Using default OUT_DIR: {}", dir);
            dir
        }
    };

    // Generate DDS module files (process only existing IDL files)
    println!("\n=== Generating DDS modules ===");
    match generate_dds_module(&out_dir, &idl_dir) {
        Ok(_) => println!("Successfully generated DDS modules"),
        Err(e) => {
            println!("Error generating DDS modules: {:?}", e);
            // Continue with build process even on failure (create empty modules)
            let modules_path = Path::new(&out_dir).join("dds_modules.rs");
            let types_path = Path::new(&out_dir).join("dds_types.rs");
            if !modules_path.exists() || !types_path.exists() {
                create_empty_modules(&out_dir)?;
            }
        }
    }

    // Create build information file
    let info_path = Path::new(&out_dir).join("dds_build_info.txt");
    let mut info_file = fs::File::create(info_path)?;
    writeln!(info_file, "DDS Build Information")?;
    writeln!(info_file, "--------------------")?;
    writeln!(info_file, "IDL Directory: {:?}", idl_dir)?;
    writeln!(info_file, "Output Directory: {}", out_dir)?;
    writeln!(info_file, "Domain ID: {}", domain_id)?;

    if idl_dir.exists() {
        let idl_files = collect_idl_files(&idl_dir)?;

        generate_type_metadata_registry(&out_dir, &idl_files)?;
        // Generate DDS type registry based on IDL files
        generate_type_registry(&out_dir, &idl_files)?;
        
        // Enable feature flag indicating type registry exists
        println!("cargo:rustc-cfg=feature=\"dds_type_registry_exists\"");
        
        writeln!(info_file, "Found IDL Files: {}", idl_files.len())?;
        for file in &idl_files {
            writeln!(info_file, "  - {:?}", file)?;
        }
    } else {
        writeln!(info_file, "IDL Directory does not exist")?;
    }

    Ok(())
}
