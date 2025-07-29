// Code generation module
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::build_scripts::idl::{collect_idl_files, IdlParser};
use crate::build_scripts::types::idl_to_rust_type;

/// Function to generate struct file
pub fn generate_struct_file(
    out_dir: &str,
    file_name: &str,
    struct_name: &str,
    fields: &HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = Path::new(out_dir).join(format!("{}.rs", file_name));
    let mut file = fs::File::create(output_path)?;

    // Write struct header with updated derive attributes
    writeln!(file, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(
        file,
        "use dust_dds::topic_definition::type_support::{{DdsType, DdsSerialize, DdsDeserialize}};"
    )?;
    writeln!(file, "")?;
    writeln!(
        file,
        "#[derive(Debug, Clone, Serialize, Deserialize, DdsType, Default)]"
    )?;
    writeln!(file, "pub struct {} {{", struct_name)?;

    // Write fields
    for (name, field_type) in fields {
        let rust_type = idl_to_rust_type(field_type);
        writeln!(file, "    pub {}: {},", name, rust_type)?;
    }

    // Close struct (removed manual impl of DdsType)
    writeln!(file, "}}")?;

    Ok(())
}

/// 타입 레지스트리 생성 함수
pub fn generate_type_registry(
    out_dir: &str,
    idl_files: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    let registry_path = Path::new(out_dir).join("dds_type_registry.rs");
    let mut registry_file = fs::File::create(&registry_path)?;

    writeln!(registry_file, "// Auto-generated DDS type registry")?;
    writeln!(registry_file, "// build.rs에 의해 생성됨")?;
    writeln!(registry_file, "")?;
    writeln!(
        registry_file,
        "use dust_dds::topic_definition::type_support::{{DdsType, DdsSerialize, DdsDeserialize}};"
    )?;
    writeln!(registry_file, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(registry_file, "use super::dds_types::*;")?;
    writeln!(registry_file, "use std::sync::Arc;")?;
    writeln!(
        registry_file,
        "use crate::vehicle::dds::listener::GenericTopicListener;"
    )?;
    writeln!(registry_file, "use crate::vehicle::dds::DdsData;")?;
    writeln!(registry_file, "")?;

    // 타입별 리스너 생성 함수
    writeln!(registry_file, "pub fn create_typed_listener(type_name: &str, topic_name: String, tx: Sender<DdsData>, domain_id: i32) -> Option<Box<dyn DdsTopicListener>> {{")?;
    writeln!(
        registry_file,
        "    println!(\"Generated - Creating listener for type: {{}}\", type_name);"
    )?;
    writeln!(registry_file, "    match type_name {{")?;

    // 각 IDL 파일에 대한 매핑 생성
    for idl_file in idl_files {
        if let Some(file_stem) = idl_file.file_stem() {
            let module_name = file_stem.to_string_lossy();

            // IDL 파일 파싱
            if let Ok(dds_data) = IdlParser::parse_idl_file(idl_file) {
                let struct_name = &dds_data.name;

                // 타입 매핑 추가
                writeln!(registry_file, "        \"{}\" => {{", struct_name)?;
                writeln!(
                    registry_file,
                    "            let listener = Box::new(GenericTopicListener::<{}::{}>::new(",
                    module_name, struct_name
                )?;
                writeln!(registry_file, "                topic_name,")?;
                writeln!(registry_file, "                type_name.to_string(),")?;
                writeln!(registry_file, "                tx,")?;
                writeln!(registry_file, "                domain_id,")?;
                writeln!(registry_file, "            ));")?;
                writeln!(registry_file, "            Some(listener)")?;
                writeln!(registry_file, "        }},")?;
            }
        }
    }

    // 기본 매핑 종료
    writeln!(registry_file, "        _ => None,")?;
    writeln!(registry_file, "    }}")?;
    writeln!(registry_file, "}}")?;

    Ok(())
}

/// Function to generate DDS module - processes only existing files
pub fn generate_dds_module(
    out_dir: &str,
    idl_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::build_scripts::idl::get_idl_files;

    // *** 여기서 get_idl_files를 명시적으로 호출 ***
    println!("Collecting IDL files using get_idl_files...");
    let idl_type_paths = get_idl_files(idl_dir)?;

    // 파일 경로 추출
    let idl_files: Vec<PathBuf> = idl_type_paths
        .iter()
        .map(|(_, path)| PathBuf::from(path))
        .collect();

    println!("Found {} IDL files from get_idl_files", idl_files.len());

    if idl_files.is_empty() {
        println!("No IDL files to process, creating minimal empty module structure");

        // Create module file (empty module, no placeholders)
        let modules_path = Path::new(out_dir).join("dds_modules.rs");
        let mut modules_file = fs::File::create(&modules_path)?;

        writeln!(modules_file, "// Auto-generated DDS module file")?;
        writeln!(modules_file, "// Generated by build.rs")?;
        writeln!(modules_file, "// Warning: No available IDL files")?;

        // Create type module (no placeholder types)
        let types_path = Path::new(out_dir).join("dds_types.rs");
        let mut types_file = fs::File::create(&types_path)?;

        writeln!(types_file, "// Auto-generated DDS type module")?;
        writeln!(types_file, "// Generated by build.rs")?;
        writeln!(types_file, "// Warning: No available IDL files")?;
        writeln!(types_file, "// This is an empty module")?;
        writeln!(types_file, "include!(\"dds_modules.rs\");")?;

        return Ok(());
    }

    // 모듈 파일 생성
    let modules_path = Path::new(out_dir).join("dds_modules.rs");
    let mut modules_file = fs::File::create(&modules_path)?;

    writeln!(modules_file, "// Auto-generated DDS module file")?;
    writeln!(modules_file, "// build.rs에 의해 생성됨")?;
    writeln!(modules_file, "")?;

    // 각 IDL 파일에 대한 모듈 생성
    for idl_file in &idl_files {
        println!("Processing IDL file: {:?}", idl_file);
        let file_stem = idl_file.file_stem().unwrap().to_string_lossy();

        // IDL 파일 파싱
        let dds_data = match IdlParser::parse_idl_file(idl_file) {
            Ok(data) => {
                println!(
                    "Successfully parsed IDL file: {} (struct: {})",
                    file_stem, data.name
                );
                data
            }
            Err(e) => {
                println!("Error parsing IDL file {}: {:?}", file_stem, e);
                continue;
            }
        };

        if dds_data.fields.is_empty() {
            println!("Warning: No fields found in struct {}", dds_data.name);
        }

        // 구조체 파일 생성
        if let Err(e) = generate_struct_file(out_dir, &file_stem, &dds_data.name, &dds_data.fields)
        {
            println!("Error generating struct file for {}: {:?}", file_stem, e);
            continue;
        }

        // 모듈에 추가
        writeln!(modules_file, "pub mod {} {{", file_stem)?;
        writeln!(modules_file, "    include!(\"{}.rs\");", file_stem)?;
        writeln!(modules_file, "}}")?;
    }

    // Create a types module that includes all the generated modules
    let types_path = Path::new(out_dir).join("dds_types.rs");
    let mut types_file = fs::File::create(&types_path)?;

    writeln!(types_file, "// Auto-generated DDS type module")?;
    writeln!(types_file, "// build.rs에 의해 생성됨")?;
    writeln!(types_file, "")?;
    writeln!(types_file, "// Include generated modules")?;
    writeln!(types_file, "include!(\"dds_modules.rs\");")?;

    println!("Successfully generated DDS modules in {}", out_dir);

    // 생성된 파일 검증
    verify_generated_files(out_dir, &modules_path, &types_path)?;

    Ok(())
}

/// 타입 메타데이터 레지스트리 생성
pub fn generate_type_metadata_registry(
    out_dir: &str,
    idl_files: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    let registry_path = Path::new(out_dir).join("dds_type_metadata.rs");
    let mut registry_file = fs::File::create(&registry_path)?;

    writeln!(registry_file, "// Auto-generated DDS type metadata")?;
    writeln!(registry_file, "use std::collections::HashMap;")?;
    writeln!(registry_file, "")?;
    writeln!(registry_file, "pub struct TypeMetadata {{")?;
    writeln!(registry_file, "    pub name: String,")?;
    writeln!(registry_file, "    pub module: String,")?;
    writeln!(registry_file, "    pub fields: HashMap<String, String>,")?;
    writeln!(registry_file, "}}")?;
    writeln!(registry_file, "")?;

    writeln!(
        registry_file,
        "pub fn get_type_metadata() -> HashMap<String, TypeMetadata> {{"
    )?;
    writeln!(registry_file, "    let mut metadata = HashMap::new();")?;
    writeln!(registry_file, "    let mut fields;")?;

    // 각 타입에 대한 메타데이터 추가
    for idl_file in idl_files {
        if let Some(file_stem) = idl_file.file_stem() {
            let module_name = file_stem.to_string_lossy();

            // IDL 파일 파싱
            if let Ok(dds_data) = IdlParser::parse_idl_file(idl_file) {
                let struct_name = &dds_data.name;

                writeln!(registry_file, "    fields = HashMap::new();")?;

                // 필드 정보 추가
                for (field_name, field_type) in &dds_data.fields {
                    let rust_type = idl_to_rust_type(field_type);
                    writeln!(
                        registry_file,
                        "    fields.insert(\"{}\".to_string(), \"{}\".to_string());",
                        field_name, rust_type
                    )?;
                }

                // 메타데이터 객체 추가
                writeln!(
                    registry_file,
                    "    metadata.insert(\"{}\".to_string(), TypeMetadata {{",
                    struct_name
                )?;
                writeln!(
                    registry_file,
                    "        name: \"{}\".to_string(),",
                    struct_name
                )?;
                writeln!(
                    registry_file,
                    "        module: \"{}\".to_string(),",
                    module_name
                )?;
                writeln!(registry_file, "        fields,")?;
                writeln!(registry_file, "    }});")?;
            }
        }
    }

    writeln!(registry_file, "    metadata")?;
    writeln!(registry_file, "}}")?;

    Ok(())
}

/// 생성된 파일 검증
pub fn verify_generated_files(
    out_dir: &str,
    modules_path: &Path,
    types_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // 파일 존재 확인
    if !modules_path.exists() || !types_path.exists() {
        println!("Warning: Expected output files were not created:");
        println!("  dds_modules.rs exists: {}", modules_path.exists());
        println!("  dds_types.rs exists: {}", types_path.exists());
        return Err("Output files were not created properly".into());
    }

    // 모듈 파일 내용 확인
    let modules_content = fs::read_to_string(modules_path)?;
    println!("dds_modules.rs size: {} bytes", modules_content.len());
    if modules_content.lines().count() < 5 {
        println!(
            "Warning: dds_modules.rs seems too short (only {} lines)",
            modules_content.lines().count()
        );
    }

    // 출력 디렉토리 내용 확인
    println!("Files in output directory:");
    for entry in fs::read_dir(Path::new(out_dir))? {
        let entry = entry?;
        println!("  {:?}", entry.path());
    }

    Ok(())
}

/// Create empty module files - no placeholders or temporary structs
pub fn create_empty_modules(out_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create dds_modules.rs
    let modules_path = Path::new(out_dir).join("dds_modules.rs");
    let mut modules_file = fs::File::create(&modules_path)?;

    writeln!(modules_file, "// Empty module (No IDL files found)")?;
    writeln!(modules_file, "// No types defined")?;

    // Create dds_types.rs
    let types_path = Path::new(out_dir).join("dds_types.rs");
    let mut types_file = fs::File::create(&types_path)?;

    writeln!(types_file, "// Empty type module (No IDL files found)")?;
    writeln!(types_file, "include!(\"dds_modules.rs\");")?;

    // Create dds_type_registry.rs (empty registry)
    let registry_path = Path::new(out_dir).join("dds_type_registry.rs");
    let mut registry_file = fs::File::create(&registry_path)?;

    writeln!(
        registry_file,
        "// Empty DDS type registry (No IDL files found)"
    )?;
    writeln!(
        registry_file,
        "use dust_dds::topic_definition::type_support::{{DdsType, DdsSerialize, DdsDeserialize}};"
    )?;
    writeln!(registry_file, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(registry_file, "use std::sync::Arc;")?;
    writeln!(
        registry_file,
        "use crate::vehicle::dds::listener::GenericTopicListener;"
    )?;
    writeln!(registry_file, "use crate::vehicle::dds::DdsData;")?;
    writeln!(registry_file, "")?;
    writeln!(registry_file, "pub fn create_typed_listener(type_name: &str, topic_name: String, tx: Sender<DdsData>, domain_id: i32) -> Option<Box<dyn DdsTopicListener>> {{")?;
    writeln!(registry_file, "    // Empty registry - always returns None")?;
    writeln!(registry_file, "    match type_name {{")?;
    writeln!(registry_file, "        _ => None,")?;
    writeln!(registry_file, "    }}")?;
    writeln!(registry_file, "}}")?;
    writeln!(registry_file, "")?;

    // Create dds_type_metadata.rs (empty metadata)
    let metadata_path = Path::new(out_dir).join("dds_type_metadata.rs");
    let mut metadata_file = fs::File::create(&metadata_path)?;

    writeln!(
        metadata_file,
        "// Empty DDS type metadata (No IDL files found)"
    )?;
    writeln!(metadata_file, "use std::collections::HashMap;")?;
    writeln!(metadata_file, "")?;
    writeln!(metadata_file, "pub struct TypeMetadata {{")?;
    writeln!(metadata_file, "    pub name: String,")?;
    writeln!(metadata_file, "    pub module: String,")?;
    writeln!(metadata_file, "    pub fields: HashMap<String, String>,")?;
    writeln!(metadata_file, "}}")?;
    writeln!(metadata_file, "")?;
    writeln!(
        metadata_file,
        "pub fn get_type_metadata() -> HashMap<String, TypeMetadata> {{"
    )?;
    writeln!(metadata_file, "    HashMap::new()  // Empty metadata")?;
    writeln!(metadata_file, "}}")?;

    println!("Created empty module files (no placeholder types)");
    Ok(())
}
