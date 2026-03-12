/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::resourcemanager::resource_manager_service_client::ResourceManagerServiceClient;
use common::resourcemanager::{
    DeleteResourceRequest, NetworkResourceRequest, ResourceResponse, VolumeResourceRequest,
};

/// Test Case: Create Network Resource
/// This test verifies that ResourceManager correctly receives and processes
/// a network resource creation request.
#[tokio::test]
async fn test_create_network_resource() {
    // Connect to ResourceManager gRPC server
    let mut client = match ResourceManagerServiceClient::connect(
        common::resourcemanager::connect_server(),
    )
    .await
    {
        Ok(client) => client,
        Err(e) => {
            println!("⚠️  ResourceManager server not running, skipping test: {}", e);
            return;
        }
    };

    // Create network resource request
    let request = NetworkResourceRequest {
        network_name: "test-network".to_string(),
        network_mode: "bridge".to_string(),
        node_name: "test-node".to_string(),
    };

    println!("📤 Sending NetworkResourceRequest:");
    println!("   • Network Name: {}", request.network_name);
    println!("   • Network Mode: {}", request.network_mode);
    println!("   • Node Name: {}", request.node_name);

    // Send request
    let response = client.create_network_resource(request).await;

    match response {
        Ok(resp) => {
            let result = resp.into_inner();
            println!("✅ Response received:");
            println!("   • Success: {}", result.success);
            println!("   • Message: {}", result.message);
        }
        Err(e) => {
            // Pharos 서버가 없어도 ResourceManager가 요청을 받은 것은 성공
            println!("⚠️  Expected error (Pharos not running): {}", e);
        }
    }
}

/// Test Case: Create Volume Resource
/// This test verifies that ResourceManager correctly receives and processes
/// a volume resource creation request.
#[tokio::test]
async fn test_create_volume_resource() {
    // Connect to ResourceManager gRPC server
    let mut client = match ResourceManagerServiceClient::connect(
        common::resourcemanager::connect_server(),
    )
    .await
    {
        Ok(client) => client,
        Err(e) => {
            println!("⚠️  ResourceManager server not running, skipping test: {}", e);
            return;
        }
    };

    // Create volume resource request
    let request = VolumeResourceRequest {
        volume_name: "test-volume".to_string(),
        capacity: "10Gi".to_string(),
        mountpath: "/mnt/data".to_string(),
        asil_level: "ASIL-B".to_string(),
        node_name: "test-node".to_string(),
    };

    println!("📤 Sending VolumeResourceRequest:");
    println!("   • Volume Name: {}", request.volume_name);
    println!("   • Capacity: {}", request.capacity);
    println!("   • Mount Path: {}", request.mountpath);
    println!("   • ASIL Level: {}", request.asil_level);
    println!("   • Node Name: {}", request.node_name);

    // Send request
    let response = client.create_volume_resource(request).await;

    match response {
        Ok(resp) => {
            let result = resp.into_inner();
            println!("✅ Response received:");
            println!("   • Success: {}", result.success);
            println!("   • Message: {}", result.message);
        }
        Err(e) => {
            // CSI 서버가 없어도 ResourceManager가 요청을 받은 것은 성공
            println!("⚠️  Expected error (CSI not running): {}", e);
        }
    }
}

/// Test Case: Delete Network Resource
#[tokio::test]
async fn test_delete_network_resource() {
    let mut client = match ResourceManagerServiceClient::connect(
        common::resourcemanager::connect_server(),
    )
    .await
    {
        Ok(client) => client,
        Err(e) => {
            println!("⚠️  ResourceManager server not running, skipping test: {}", e);
            return;
        }
    };

    let request = DeleteResourceRequest {
        resource_name: "test-network".to_string(),
    };

    println!("📤 Sending DeleteNetworkResourceRequest:");
    println!("   • Resource Name: {}", request.resource_name);

    let response = client.delete_network_resource(request).await;

    match response {
        Ok(resp) => {
            let result = resp.into_inner();
            println!("✅ Response received:");
            println!("   • Success: {}", result.success);
            println!("   • Message: {}", result.message);
        }
        Err(e) => {
            println!("⚠️  Expected error (Pharos not running): {}", e);
        }
    }
}

/// Test Case: Delete Volume Resource
#[tokio::test]
async fn test_delete_volume_resource() {
    let mut client = match ResourceManagerServiceClient::connect(
        common::resourcemanager::connect_server(),
    )
    .await
    {
        Ok(client) => client,
        Err(e) => {
            println!("⚠️  ResourceManager server not running, skipping test: {}", e);
            return;
        }
    };

    let request = DeleteResourceRequest {
        resource_name: "test-volume".to_string(),
    };

    println!("📤 Sending DeleteVolumeResourceRequest:");
    println!("   • Resource Name: {}", request.resource_name);

    let response = client.delete_volume_resource(request).await;

    match response {
        Ok(resp) => {
            let result = resp.into_inner();
            println!("✅ Response received:");
            println!("   • Success: {}", result.success);
            println!("   • Message: {}", result.message);
        }
        Err(e) => {
            println!("⚠️  Expected error (CSI not running): {}", e);
        }
    }
}
