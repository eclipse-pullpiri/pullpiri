/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(
            &["proto/apiserver.proto", "proto/actioncontroller.proto"],
            &["proto"],
        )?;
    tonic_build::compile_protos("proto/filtergateway.proto")?;
    tonic_build::compile_protos("proto/monitoringserver.proto")?;
    tonic_build::compile_protos("proto/nodeagent.proto")?;
    tonic_build::compile_protos("proto/policymanager.proto")?;
    tonic_build::compile_protos("proto/statemanager.proto")?;
    tonic_build::compile_protos("proto/pharos_service.proto")?;
    Ok(())
}
