/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/constants.proto")?;
    tonic_build::compile_protos("proto/apiserver.proto")?;
    tonic_build::compile_protos("proto/apiserver/metric_notifier.proto")?;
    tonic_build::compile_protos("proto/statemanager.proto")?;
    tonic_build::compile_protos("proto/gateway.proto")?;
    Ok(())
}
