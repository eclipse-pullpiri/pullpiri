/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::Path::new("src/generated");
    if !out_dir.exists() {
        std::fs::create_dir_all(out_dir)?;
    }

    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .protoc_arg("--experimental_allow_proto3_optional")
        .out_dir(out_dir)
        .compile_protos(
            &[
                "proto/apiserver.proto",
                "proto/actioncontroller.proto",
                "proto/filtergateway.proto",
                "proto/monitoringserver.proto",
                "proto/policymanager.proto",
                "proto/statemanager.proto",
                "proto/nodeagent.proto",
                "proto/logd.proto",
                "proto/external/pharos/pharos_service.proto",
                "proto/external/timpani/schedinfo.proto",
                "proto/rocksdbservice.proto", // Add RocksDB service proto
                "proto/resourcemanager.proto", // Add Resource Manager proto
                "proto/external/csi/csi_service.proto", // Add CSI service proto
            ],
            &["proto"],
        )?;
    Ok(())
}
