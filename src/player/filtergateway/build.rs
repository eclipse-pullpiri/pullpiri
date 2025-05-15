use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use serde_yaml;

// Define DdsData structure in build.rs to avoid having to import it
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdsData {
    pub name: String,
    pub value: String,
    pub fields: HashMap<String, String>,
}

/// IDL 파서 구현
pub struct IdlParser;

impl IdlParser {
    /// IDL 파일 파싱
    pub fn parse_idl_file(file_path: &Path) -> Result<DdsData, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        let mut lines = content.lines();

        // Struct name extraction
        let mut struct_name = String::new();
        let mut fields = HashMap::new();

        // Find struct definition
        while let Some(line) = lines.next() {
            let line = line.trim();

            // Look for struct definition
            if let Some(pos) = line.find("struct") {
                let remaining = &line[pos + "struct".len()..].trim();
                if let Some(end_pos) = remaining.find('{') {
                    struct_name = remaining[..end_pos].trim().to_string();
                    break;
                } else {
                    struct_name = remaining.to_string();
                    break;
                }
            }
        }

        // Find fields
        let mut inside_struct = false;
        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            if !inside_struct && line.contains('{') {
                inside_struct = true;
                continue;
            }

            if inside_struct {
                if line.contains('}') {
                    break;
                }

                // Parse field
                let line = line.trim_end_matches(';');
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let field_type = parts[0].to_string();
                    let field_name = parts[1].to_string();
                    fields.insert(field_name, field_type);
                }
            }
        }

        Ok(DdsData {
            name: struct_name,
            value: "{}".to_string(), // Default empty JSON
            fields,
        })
    }
}

/// 구조체 파일 생성 함수
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
fn generate_type_registry(out_dir: &str, idl_files: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    let registry_path = Path::new(out_dir).join("dds_type_registry.rs");
    let mut registry_file = fs::File::create(&registry_path)?;
    
    writeln!(registry_file, "// 자동 생성된 DDS 타입 레지스트리")?;
    writeln!(registry_file, "// build.rs에 의해 생성됨")?;
    writeln!(registry_file, "")?;
    writeln!(registry_file, "use dust_dds::topic_definition::type_support::{{DdsType, DdsSerialize, DdsDeserialize}};")?;
    writeln!(registry_file, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(registry_file, "use super::dds_types::*;")?;
    writeln!(registry_file, "use std::sync::Arc;")?;
    writeln!(registry_file, "use crate::vehicle::dds::listener::GenericTopicListener;")?;
    // writeln!(registry_file, "use crate::vehicle::dds::DdsTopicListener;")?;
    // writeln!(registry_file, "use tokio::sync::mpsc::Sender;")?;
    writeln!(registry_file, "use crate::vehicle::dds::DdsData;")?;
    writeln!(registry_file, "")?;
    
    // 타입별 리스너 생성 함수
    writeln!(registry_file, "pub fn create_typed_listener(type_name: &str, topic_name: String, tx: Sender<DdsData>, domain_id: i32) -> Option<Box<dyn DdsTopicListener>> {{")?;
    writeln!(registry_file, "    println!(\"Generated - Creating listener for type: {{}}\", type_name);")?;
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
                writeln!(registry_file, "            let listener = Box::new(GenericTopicListener::<{}::{}>::new(", module_name, struct_name)?;
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
    
    // 가능한 타입 목록
    // writeln!(registry_file, "")?;
    // writeln!(registry_file, "pub fn get_available_types() -> Vec<String> {{")?;
    // writeln!(registry_file, "    vec![")?;
    
    // for idl_file in idl_files {
    //     if let Ok(dds_data) = IdlParser::parse_idl_file(idl_file) {
    //         writeln!(registry_file, "        \"{}\".to_string(),", dds_data.name)?;
    //     }
    // }
    
    // writeln!(registry_file, "    ]")?;
    // writeln!(registry_file, "}}")?;
    
    Ok(())
}

/// IDL 타입을 Rust 타입으로 변환
fn idl_to_rust_type(idl_type: &str) -> &str {
    match idl_type {
        "boolean" => "bool",
        "short" | "int16_t" => "i16",
        "unsigned short" | "uint16_t" => "u16",
        "long" | "int32_t" => "i32",
        "unsigned long" | "uint32_t" => "u32",
        "long long" | "int64_t" => "i64",
        "unsigned long long" | "uint64_t" => "u64",
        "float" => "f32",
        "double" => "f64",
        "string" | "std::string" => "String",
        "octet" | "byte" => "u8",
        "char" => "char",
        _ => "String", // Default to String for complex types
    }
}

/// IDL 파일 수집 함수 - 존재하는 파일만 읽음, 생성하지 않음
fn collect_idl_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut idl_files = Vec::new();

    if !dir.exists() {
        println!("IDL directory does not exist: {:?}", dir);
        return Ok(idl_files); // 디렉토리가 없으면 빈 벡터 반환
    }

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "idl") {
                println!("Found IDL file: {:?}", path);
                idl_files.push(path);
            }
        }
    }

    if idl_files.is_empty() {
        println!("No IDL files found in directory: {:?}", dir);
    } else {
        println!(
            "Found {} IDL files in directory: {:?}",
            idl_files.len(),
            dir
        );
    }

    Ok(idl_files)
}

/// IDL 파일 메타데이터 가져오기
fn get_idl_files(dir: &Path) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let mut result = Vec::new();

    println!(
        "*** get_idl_files is being called for directory: {:?} ***",
        dir
    );

    if !dir.exists() {
        println!("Directory does not exist: {:?}", dir);
        return Ok(result);
    }

    // 디렉토리 내용 출력
    println!("Directory contents:");
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        println!("  - {:?} (is_file: {})", path, path.is_file());

        if path.is_file() && path.extension().map_or(false, |ext| ext == "idl") {
            if let Some(stem) = path.file_stem() {
                let type_name = stem.to_string_lossy().to_string();
                let file_path = path.to_string_lossy().to_string();
                println!("  * Found IDL file: {} at {}", type_name, file_path);
                result.push((type_name, file_path));
            }
        }
    }

    println!("*** get_idl_files found {} IDL files ***", result.len());
    Ok(result)
}

/// DDS 모듈 생성 함수 - 존재하는 파일만 처리
fn generate_dds_module(out_dir: &str, idl_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
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
        
        // 모듈 파일 생성 (빈 모듈, 플레이스홀더 없음)
        let modules_path = Path::new(out_dir).join("dds_modules.rs");
        let mut modules_file = fs::File::create(&modules_path)?;
        
        writeln!(modules_file, "// 자동 생성된 DDS 모듈 파일")?;
        writeln!(modules_file, "// build.rs에 의해 생성됨")?;
        writeln!(modules_file, "// 주의: 사용 가능한 IDL 파일이 없습니다")?;
        
        // 타입 모듈 생성 (플레이스홀더 타입 없음)
        let types_path = Path::new(out_dir).join("dds_types.rs");
        let mut types_file = fs::File::create(&types_path)?;
        
        writeln!(types_file, "// 자동 생성된 DDS 타입 모듈")?;
        writeln!(types_file, "// build.rs에 의해 생성됨")?;
        writeln!(types_file, "// 주의: 사용 가능한 IDL 파일이 없습니다")?;
        writeln!(types_file, "// 비어있는 모듈입니다")?;
        writeln!(types_file, "include!(\"dds_modules.rs\");")?;
        
        return Ok(());
    }

    // 모듈 파일 생성
    let modules_path = Path::new(out_dir).join("dds_modules.rs");
    let mut modules_file = fs::File::create(&modules_path)?;

    writeln!(modules_file, "// 자동 생성된 DDS 모듈 파일")?;
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

    writeln!(types_file, "// 자동 생성된 DDS 타입 모듈")?;
    writeln!(types_file, "// build.rs에 의해 생성됨")?;
    writeln!(types_file, "")?;
    writeln!(types_file, "// Include generated modules")?;
    writeln!(types_file, "include!(\"dds_modules.rs\");")?;

    println!("Successfully generated DDS modules in {}", out_dir);

    // 생성된 파일 검증
    verify_generated_files(out_dir, &modules_path, &types_path)?;

    Ok(())
}

/// 생성된 파일 검증
fn verify_generated_files(
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

/// IDL 파일을 로드하여 DdsData 구조체로 반환
fn load_idl_file(path: &Path) -> Result<DdsData, Box<dyn std::error::Error>> {
    IdlParser::parse_idl_file(path)
}

/// 타입 메타데이터 레지스트리 생성
fn generate_type_metadata_registry(out_dir: &str, idl_files: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    let registry_path = Path::new(out_dir).join("dds_type_metadata.rs");
    let mut registry_file = fs::File::create(&registry_path)?;
    
    writeln!(registry_file, "// 자동 생성된 DDS 타입 메타데이터")?;
    writeln!(registry_file, "use std::collections::HashMap;")?;
    writeln!(registry_file, "")?;
    writeln!(registry_file, "pub struct TypeMetadata {{")?;
    writeln!(registry_file, "    pub name: String,")?;
    writeln!(registry_file, "    pub module: String,")?;
    writeln!(registry_file, "    pub fields: HashMap<String, String>,")?;
    writeln!(registry_file, "}}")?;
    writeln!(registry_file, "")?;
    
    writeln!(registry_file, "pub fn get_type_metadata() -> HashMap<String, TypeMetadata> {{")?;
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
                writeln!(registry_file, "        name: \"{}\".to_string(),", struct_name)?;
                writeln!(registry_file, "        module: \"{}\".to_string(),", module_name)?;
                writeln!(registry_file, "        fields,")?;
                writeln!(registry_file, "    }});")?;
            }
        }
    }
    
    writeln!(registry_file, "    metadata")?;
    writeln!(registry_file, "}}")?;
    
    Ok(())
}

/// settings.yaml에서 DDS 설정 로드 함수 (빌드 시)
fn load_dds_settings() -> Result<(PathBuf, i32, Option<String>), Box<dyn std::error::Error>> {
    // 기본값 설정 (설정 파일이 없는 경우 사용)
    // 프로젝트 루트 기준 경로로 변경
    let default_idl_dir = PathBuf::from("src/vehicle/dds/idl");
    let default_domain_id = 0;
    let default_out_dir = None; // 기본값은 환경 변수 OUT_DIR 사용

    // 프로젝트 루트 기준으로 설정 파일 경로 검색
    let mut settings_path = PathBuf::from("/home/edo/2025/projects/pullpiri/src/settings.yaml");
    let mut settings_content = String::new();



    if !settings_path.exists() {
        println!("No settings file found, using defaults");
        return Ok((default_idl_dir, default_domain_id, default_out_dir));
    }
    
    

    // 설정 파일 읽기
    println!("Reading settings from: {:?}", settings_path);
    settings_content = fs::read_to_string(&settings_path)?;

    // JSON 또는 YAML 파싱
    let settings: serde_yaml::Value =  serde_yaml::from_str(&settings_content)?;
 

    println!("Settings content: {}", settings_content);

    // DDS 설정 추출 (프로젝트 루트 기준의 상대 경로)
    let idl_path = settings
        .get("dds")
        .and_then(|dds| dds.get("idl_path"))
        .and_then(|path| path.as_str())
        .map(PathBuf::from)
        .unwrap_or(default_idl_dir);

    // 프로젝트 루트 기준의 절대 경로 계산
    println!("IDL path from settings (relative): {:?}", idl_path);

    let domain_id = settings
        .get("dds")
        .and_then(|dds| dds.get("domain_id"))
        .and_then(|id| id.as_i64())
        .map(|id| id as i32)
        .unwrap_or(default_domain_id);

    println!("Domain ID from settings: {}", domain_id);

    // 사용자 정의 OUT_DIR 값 확인 (절대 경로 또는 프로젝트 루트 기준 상대 경로)
    let out_dir = settings
        .get("dds")
        .and_then(|dds| dds.get("out_dir"))
        .and_then(|path| path.as_str())
        .map(String::from);

    if let Some(dir) = &out_dir {
        println!("Output directory from settings: {}", dir);
    }

    Ok((idl_path, domain_id, out_dir))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/vehicle/dds/idl");
    println!("cargo:rerun-if-changed=/home/edo/2025/projects/pullpiri/src/settings.yaml");

    // 현재 작업 디렉토리 (프로젝트 루트) 확인
    let current_dir = env::current_dir().expect("현재 디렉토리를 확인할 수 없습니다");
    println!("Current working directory: {:?}", current_dir);

    // 설정 로드
    let (rel_idl_dir, domain_id, custom_out_dir) = load_dds_settings()?;

    // 상대 경로를 현재 작업 디렉토리 기준 절대 경로로 변환
    let idl_dir = current_dir.join(&rel_idl_dir);
    println!("IDL directory (absolute): {:?}", idl_dir);
    println!("Domain ID: {}", domain_id);

    // IDL 디렉토리 확인 (생성하지 않음)
    if !idl_dir.exists() {
        println!("Warning: IDL directory doesn't exist: {:?}", idl_dir);
        println!("No files will be processed");
    } else {
        // 절대 경로 출력 (디버깅용)
        if let Ok(abs_path) = idl_dir.canonicalize() {
            println!("IDL directory absolute path: {:?}", abs_path);
        }
    }

    // IDL 파일 명시적으로 검색
    println!("\n=== Explicitly calling get_idl_files to find IDL files ===");
    let idl_files_result = get_idl_files(&idl_dir);
    match idl_files_result {
        Ok(files) => {
            println!("Found {} IDL files:", files.len());
            for (name, path) in files {
                println!("  - {} at {}", name, path);
            }
        }
        Err(e) => println!("Error getting IDL files: {:?}", e),
    }

    // 출력 디렉토리 설정 (settings.yaml에서 지정한 경우 사용)
    let out_dir = match custom_out_dir {
        Some(dir) => {
            // 사용자 정의 디렉토리가 있으면 해당 디렉토리 사용
            let path = PathBuf::from(&dir);
            println!("Using custom output directory: {}", path.display());

            // 디렉토리가 없으면 생성
            if !path.exists() {
                println!("Creating custom output directory");
                fs::create_dir_all(&path)?;
            }

            // 환경 변수 OUT_DIR 재설정 (다른 빌드 스크립트에서도 사용할 수 있도록)
            println!("cargo:rustc-env=OUT_DIR={}", path.display());

            dir
        }
        None => {
            // 기본 OUT_DIR 환경 변수 사용
            let dir = env::var("OUT_DIR").expect("OUT_DIR 환경 변수를 찾을 수 없습니다");
            println!("Using default OUT_DIR: {}", dir);
            dir
        }
    };

    // DDS 모듈 파일 생성 (존재하는 IDL 파일만 처리)
    println!("\n=== Generating DDS modules ===");
    match generate_dds_module(&out_dir, &idl_dir) {
        Ok(_) => println!("Successfully generated DDS modules"),
        Err(e) => {
            println!("Error generating DDS modules: {:?}", e);
            // 빌드 실패하지 않고 계속 진행 (빈 모듈 생성)
            let modules_path = Path::new(&out_dir).join("dds_modules.rs");
            let types_path = Path::new(&out_dir).join("dds_types.rs");
            if !modules_path.exists() || !types_path.exists() {
                create_empty_modules(&out_dir)?;
            }
        }
    }

    // 빌드 정보 파일 생성
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
        // IDL 파일을 기반으로 DDS 타입 레지스트리 생성
        generate_type_registry(&out_dir, &idl_files)?;
        
        // 타입 레지스트리가 존재함을 표시하는 피처 활성화
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

/// 빈 모듈 파일 생성 - 플레이스홀더나 임시 구조체 없음
fn create_empty_modules(out_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // dds_modules.rs 생성
    let modules_path = Path::new(out_dir).join("dds_modules.rs");
    let mut modules_file = fs::File::create(&modules_path)?;
    
    writeln!(modules_file, "// 빈 모듈 (IDL 파일을 찾을 수 없음)")?;
    writeln!(modules_file, "// 아무 타입도 정의되지 않음")?;
    
    // dds_types.rs 생성
    let types_path = Path::new(out_dir).join("dds_types.rs");
    let mut types_file = fs::File::create(&types_path)?;
    
    writeln!(types_file, "// 빈 타입 모듈 (IDL 파일을 찾을 수 없음)")?;
    writeln!(types_file, "include!(\"dds_modules.rs\");")?;
    
    // dds_type_registry.rs 생성 (빈 레지스트리)
    let registry_path = Path::new(out_dir).join("dds_type_registry.rs");
    let mut registry_file = fs::File::create(&registry_path)?;
    
    writeln!(registry_file, "// 빈 DDS 타입 레지스트리 (IDL 파일을 찾을 수 없음)")?;
    writeln!(registry_file, "use dust_dds::topic_definition::type_support::{{DdsType, DdsSerialize, DdsDeserialize}};")?;
    writeln!(registry_file, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(registry_file, "use std::sync::Arc;")?;
    writeln!(registry_file, "use crate::vehicle::dds::listener::GenericTopicListener;")?;
    // writeln!(registry_file, "use crate::vehicle::dds::DdsTopicListener;")?;
    // writeln!(registry_file, "use tokio::sync::mpsc::Sender;")?;
    writeln!(registry_file, "use crate::vehicle::dds::DdsData;")?;
    writeln!(registry_file, "")?;
    writeln!(registry_file, "pub fn create_typed_listener(type_name: &str, topic_name: String, tx: Sender<DdsData>, domain_id: i32) -> Option<Box<dyn DdsTopicListener>> {{")?;
    writeln!(registry_file, "    // 비어있는 레지스트리 - 항상 None 반환")?;
    writeln!(registry_file, "    match type_name {{")?;
    writeln!(registry_file, "        _ => None,")?;
    writeln!(registry_file, "    }}")?;
    writeln!(registry_file, "}}")?;
    writeln!(registry_file, "")?;
    // writeln!(registry_file, "pub fn get_available_types() -> Vec<String> {{")?;
    // writeln!(registry_file, "    vec![]  // 빈 타입 목록")?;
    // writeln!(registry_file, "}}")?;
    
    // dds_type_metadata.rs 생성 (빈 메타데이터)
    let metadata_path = Path::new(out_dir).join("dds_type_metadata.rs");
    let mut metadata_file = fs::File::create(&metadata_path)?;
    
    writeln!(metadata_file, "// 빈 DDS 타입 메타데이터 (IDL 파일을 찾을 수 없음)")?;
    writeln!(metadata_file, "use std::collections::HashMap;")?;
    writeln!(metadata_file, "")?;
    writeln!(metadata_file, "pub struct TypeMetadata {{")?;
    writeln!(metadata_file, "    pub name: String,")?;
    writeln!(metadata_file, "    pub module: String,")?;
    writeln!(metadata_file, "    pub fields: HashMap<String, String>,")?;
    writeln!(metadata_file, "}}")?;
    writeln!(metadata_file, "")?;
    writeln!(metadata_file, "pub fn get_type_metadata() -> HashMap<String, TypeMetadata> {{")?;
    writeln!(metadata_file, "    HashMap::new()  // 빈 메타데이터")?;
    writeln!(metadata_file, "}}")?;
    
    println!("Created empty module files (no placeholder types)");
    Ok(())
}
