/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
use ssh2::Session;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub fn get_absolute_file_path(path: &str) -> Result<PathBuf, Box<dyn Error>> {
    let file = Path::new(path);
    if !file.is_file() {
        return Err(format!("Not found or invalid path - {path}").into());
    }

    if matches!(file.extension(), Some(x) if x == OsStr::new("yaml")) {
        Ok(std::fs::canonicalize(file)?)
    } else {
        Err("Unsupported file extension".into())
    }
}

fn make_kube_file(dir: &str, name: &str) -> Result<(), Box<dyn Error>> {
    let kube_file_path = format!("{}/{}.kube", dir, name);
    let yaml_file_path = format!("{}/{}.yaml", dir, name);
    let mut kube_file = File::create(kube_file_path)?;

    let kube_contents = format!(
        r#"[Unit]
Description=A kubernetes yaml based {} service
After=network.target

[Install]
# Start by default on boot
WantedBy=multi-user.target default.target

[Kube]
Yaml={}

[Service]
Restart=no
"#,
        name, yaml_file_path
    );
    kube_file.write_all(kube_contents.as_bytes())?;

    Ok(())
}

fn make_yaml_file(dir: &str, name: &str, model: &str) -> Result<(), Box<dyn Error>> {
    let yaml_file_path = format!("{}/{}.yaml", dir, name);
    let mut yaml_file = File::create(yaml_file_path)?;

    yaml_file.write_all(model.as_bytes())?;

    Ok(())
}

pub fn perform(
    model_name: &str,
    parsed_model_str: &str,
    package_name: &str,
) -> Result<(), Box<dyn Error>> {
    let directory = format!(
        "{}/packages/{}/{}",
        common::get_config().yaml_storage,
        package_name,
        model_name
    );
    std::fs::create_dir_all(&directory)?;

    make_kube_file(&directory, model_name)?;
    make_yaml_file(&directory, model_name, parsed_model_str)?;

    Ok(())
}

pub async fn download(url: &str, path: &str) -> common::Result<()> {
    let username = "id";
    let password = Some("pw".to_string());
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .basic_auth(username, password)
        .send()
        .await?;
    //let response = reqwest::get(url).await?;

    if response.status().is_success() {
        let mut file = BufWriter::new(File::create(path)?);
        let content = response.bytes().await?;
        std::io::copy(&mut content.as_ref(), &mut file)?;
        println!("File downloaded to {}", path);
        Ok(())
    } else {
        Err(format!("Failed to download file: {}", response.status()).into())
    }
}

pub fn extract(path: &str) -> Result<(), Box<dyn Error>> {
    let file = File::open(path)?;
    let mut archive: tar::Archive<File> = tar::Archive::new(file);

    let destination = Path::new(path).parent().unwrap();
    archive.unpack(destination)?;
    println!("TAR file extracted to {:?}", destination);
    Ok(())
}

pub fn create_exist_folder(path: &str) -> common::Result<()> {
    if !Path::new(path).exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn copy_to_remote_node(path: &str) -> common::Result<()> {
    if let Some(guests) = &common::get_config().guest {
        for guest in guests {
            let guest_ssh_ip = format!("{}:{}", guest.ip, guest.ssh_port);
            let tcp = std::net::TcpStream::connect(guest_ssh_ip)?;
            let mut session = Session::new()?;

            session.set_tcp_stream(tcp);
            session.handshake()?;
            session.userauth_password(&guest.id, &guest.pw)?;
            if !session.authenticated() {
                println!("auth failed to remote node");
                return Err("auth failed".into());
            }

            session.sftp()?.mkdir(&PathBuf::from(path), 0o755)?;
            recursive_copy(&session, &PathBuf::from(path))?;
        }
    } else {
        println!("There is no guest node.");
    }

    Ok(())
}

fn recursive_copy(session: &Session, path: &Path) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            session.sftp()?.mkdir(&entry_path, 0o755)?;
            recursive_copy(session, &entry_path)?;
        } else {
            // entry_path is file
            let mut guest_file = session.sftp()?.create(&entry_path)?;
            let mut host_file = fs::File::open(&entry_path)?;
            io::copy(&mut host_file, &mut guest_file)?;
        }
    }
    Ok(())
}
