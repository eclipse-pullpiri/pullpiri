// Module for IDL parsing
use crate::build_scripts::types::DdsData;
use std::fs;
use std::path::{Path, PathBuf};

/// IDL parser implementation
pub struct IdlParser;

impl IdlParser {
    /// Parse IDL file
    pub fn parse_idl_file(file_path: &Path) -> Result<DdsData, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        let mut lines = content.lines();

        // Struct name extraction
        let mut struct_name = String::new();
        let mut fields = std::collections::HashMap::new();

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

/// IDL 파일 수집 함수 - 존재하는 파일만 읽음, 생성하지 않음
pub fn collect_idl_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
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
pub fn get_idl_files(dir: &Path) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
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

/// IDL 파일을 로드하여 DdsData 구조체로 반환
pub fn load_idl_file(path: &Path) -> Result<DdsData, Box<dyn std::error::Error>> {
    IdlParser::parse_idl_file(path)
}
