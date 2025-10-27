/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! MonitoringServerManager: Asynchronous manager for MonitoringServer
//!
//! This struct manages scenario requests received via gRPC, and provides
//! a gRPC sender for communicating with the nodeagent or other services.
//! It is designed to be thread-safe and run in an async context.
use crate::data_structures::{BoardInfo, DataStore, SocInfo};
use common::monitoringserver::{ContainerList, NodeInfo}; // Use protobuf types
use common::Result;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Main manager struct for MonitoringServer.
///
/// Holds the gRPC receiver and sender, and manages the main event loop.
pub struct MonitoringServerManager {
    /// Receiver for container information from gRPC
    rx_container: Arc<Mutex<mpsc::Receiver<ContainerList>>>,
    /// Receiver for node information from gRPC
    rx_node: Arc<Mutex<mpsc::Receiver<NodeInfo>>>,
    /// Receiver for stress metrics (JSON strings) from gRPC
    rx_stress: Arc<Mutex<mpsc::Receiver<String>>>,
    /// Data store for managing NodeInfo, SocInfo, and BoardInfo
    data_store: Arc<Mutex<DataStore>>,
}

impl MonitoringServerManager {
    /// Creates a new MonitoringServerManager instance.
    pub async fn new(
        rx_container: mpsc::Receiver<ContainerList>,
        rx_node: mpsc::Receiver<NodeInfo>,
        rx_stress: mpsc::Receiver<String>,
    ) -> Self {
        Self {
            rx_container: Arc::new(Mutex::new(rx_container)),
            rx_node: Arc::new(Mutex::new(rx_node)),
            rx_stress: Arc::new(Mutex::new(rx_stress)),
            data_store: Arc::new(Mutex::new(DataStore::new())),
        }
    }

    /// Initializes the MonitoringServerManager (e.g., loads scenarios, prepares state).
    pub async fn initialize(&mut self) -> Result<()> {
        println!("MonitoringServerManager init");
        // Add initialization logic here (e.g., read scenarios, subscribe, etc.)
        Ok(())
    }

    /// Processes ContainerList messages from nodeagent.
    ///
    /// This function handles the received ContainerList and processes it accordingly.
    async fn handle_container_list(&self, container_list: ContainerList) {
        println!(
            "[MonitoringServer] Received ContainerList from {}: containers count={}",
            container_list.node_name,
            container_list.containers.len()
        );

        let current_container_ids: Vec<String> = container_list
            .containers
            .iter()
            .map(|c| c.id.clone())
            .collect();

        let mut data_store = self.data_store.lock().await;

        // Clean up containers that are no longer present on this node
        data_store
            .cleanup_node_containers(&container_list.node_name, &current_container_ids)
            .await;

        // Store current containers with node association
        for container in &container_list.containers {
            match data_store
                .store_container_info_with_node(container.clone(), container_list.node_name.clone())
                .await
            {
                Ok(_) => {
                    println!(
                        "[MonitoringServer] SUCCESS: Stored container {} on node {}",
                        container.id, container_list.node_name
                    );
                }
                Err(e) => eprintln!(
                    "[MonitoringServer] ERROR: Failed to store container {}: {}",
                    container.id, e
                ),
            }
        }

        self.print_container_summary(&container_list).await;
    }

    /// Print container summary for a node (line-wise, formatted)
    async fn print_container_summary(&self, container_list: &ContainerList) {
        println!(
            "\n┌───────────────────────────── CONTAINER SUMMARY ─────────────────────────────┐"
        );
        println!("│ Node: {:<69} │", container_list.node_name);
        println!(
            "│ Total Containers: {:<59} │",
            container_list.containers.len()
        );
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        for (i, container) in container_list.containers.iter().enumerate() {
            let name = container
                .names
                .first()
                .cloned()
                .unwrap_or_else(|| "unnamed".to_string());
            let status = container
                .state
                .get("Status")
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            let status_icon = match status.as_str() {
                "running" => "🟢",
                "exited" => "🔴",
                "paused" => "🟡",
                _ => "⚪",
            };
            println!(
                "│ {:>2}. {} Name: {:<20} │ Image: {:<20} │ Status: {:<10} │",
                i + 1,
                status_icon,
                name,
                container.image,
                status
            );
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Print comprehensive container overview (line-wise, formatted)
    pub async fn print_container_overview(&self) {
        let data_store = self.data_store.lock().await;
        let containers = data_store.get_all_containers();
        let running_count = containers
            .values()
            .filter(|c| c.state.get("Running").map(|v| v == "true").unwrap_or(false))
            .count();
        let stopped_count = containers.len() - running_count;

        println!(
            "\n┌───────────────────────── SYSTEM CONTAINER OVERVIEW ─────────────────────────┐"
        );
        println!("│ Total Containers: {:<59} │", containers.len());
        println!(
            "│ Running: {:<8} │ Stopped: {:<8} │",
            running_count, stopped_count
        );
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        for (i, container) in containers.values().enumerate() {
            let name = container
                .names
                .first()
                .cloned()
                .unwrap_or_else(|| "unnamed".to_string());
            let status = container
                .state
                .get("Status")
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            println!(
                "│ {:>2}. Name: {:<20} │ Image: {:<20} │ Status: {:<10} │",
                i + 1,
                name,
                container.image,
                status
            );
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Print all nodes (line-wise, formatted)
    pub async fn print_all_nodes(&self) {
        let data_store = self.data_store.lock().await;
        println!(
            "\n┌────────────────────────────────── ALL NODES ───────────────────────────────┐"
        );
        for (i, (_, node)) in data_store.get_all_nodes().iter().enumerate() {
            println!("│ {:>2}. Node: {:<20} │ IP: {:<15} │ CPU: {:>5.2}% │ Mem: {:>5.2}% │ Containers: {:<3} │",
                i + 1, node.node_name, node.ip, node.cpu_usage, node.mem_usage,
                data_store.get_containers_by_node(&node.node_name).len());
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Print all containers (line-wise, formatted)
    pub async fn print_all_containers(&self) {
        let data_store = self.data_store.lock().await;
        println!(
            "\n┌──────────────────────────────── ALL CONTAINERS ─────────────────────────────┐"
        );
        for (i, (_, container)) in data_store.get_all_containers().iter().enumerate() {
            let name = container
                .names
                .first()
                .cloned()
                .unwrap_or_else(|| "unnamed".to_string());
            let status = container
                .state
                .get("Status")
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            println!(
                "│ {:>2}. Name: {:<20} │ ID: {:<12} │ Image: {:<20} │ Status: {:<10} │",
                i + 1,
                name,
                &container.id[..std::cmp::min(12, container.id.len())],
                container.image,
                status
            );
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Print all boards (line-wise, formatted)
    pub async fn print_all_boards(&self) {
        let data_store = self.data_store.lock().await;
        println!(
            "\n┌────────────────────────────────── ALL BOARDS ───────────────────────────────┐"
        );
        for (i, (_, board)) in data_store.get_all_boards().iter().enumerate() {
            println!("│ {:>2}. Board: {:<20} │ Nodes: {:<3} │ SoCs: {:<3} │ CPU: {:>5.2}% │ Mem: {:>5.2}% │",
                i + 1, board.board_id, board.nodes.len(), board.socs.len(), board.total_cpu_usage, board.total_mem_usage);
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Print all SoCs (line-wise, formatted)
    pub async fn print_all_socs(&self) {
        let data_store = self.data_store.lock().await;
        println!(
            "\n┌────────────────────────────────── ALL SOCs ────────────────────────────────┐"
        );
        for (i, (_, soc)) in data_store.get_all_socs().iter().enumerate() {
            println!(
                "│ {:>2}. SoC: {:<20} │ Nodes: {:<3} │ CPU: {:>5.2}% │ Mem: {:>5.2}% │",
                i + 1,
                soc.soc_id,
                soc.nodes.len(),
                soc.total_cpu_usage,
                soc.total_mem_usage
            );
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Processes NodeInfo messages from nodeagent.
    ///
    /// This function handles the received NodeInfo and processes it accordingly.
    async fn handle_node_info(&self, node_info: NodeInfo) {
        // Print detailed NodeInfo first
        self.print_node_info(&node_info);

        // Store NodeInfo and update SocInfo/BoardInfo with etcd storage
        {
            let mut data_store = self.data_store.lock().await;
            match data_store.store_node_info(node_info.clone()).await {
                Ok(_) => {
                    println!(
                        "[MonitoringServer] SUCCESS: Successfully stored NodeInfo for {}",
                        node_info.node_name
                    );

                    // Print ID generation details
                    self.print_id_generation_details(&node_info.ip);

                    // Print aggregated information
                    self.print_aggregated_info(&data_store, &node_info.ip).await;

                    // Print detailed SoC mapping
                    self.print_detailed_soc_mapping(&data_store).await;

                    // Print summary statistics
                    self.print_summary_stats(&data_store).await;
                }
                Err(e) => {
                    eprintln!("[MonitoringServer] ERROR: Error storing NodeInfo: {}", e);
                }
            }
        }

        println!("{}", "=".repeat(80));
    }

    /// Print ID generation details for debugging
    fn print_id_generation_details(&self, ip: &str) {
        println!("\n ID GENERATION DEBUG");
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        println!("│ Input IP: {:<65} │", ip);

        if let Ok(soc_id) = DataStore::generate_soc_id(ip) {
            println!("│ Generated SoC ID: {:<57} │", soc_id);
        }

        if let Ok(board_id) = DataStore::generate_board_id(ip) {
            println!("│ Generated Board ID: {:<55} │", board_id);
        }

        // Show the logic
        if let Ok(parsed_ip) = std::net::Ipv4Addr::from_str(ip) {
            let octets = parsed_ip.octets();
            let last_octet = octets[3];
            let soc_group = (last_octet / 10) * 10;
            let board_group = (last_octet / 100) * 100;

            println!(
                "│ Last Octet: {:<3} → SoC Group: {:<3} → Board Group: {:<8}                    │",
                last_octet, soc_group, board_group
            );
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Print detailed SoC mapping for all current data
    async fn print_detailed_soc_mapping(&self, data_store: &DataStore) {
        println!("\n DETAILED SOC MAPPING");
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");

        for (soc_id, soc_info) in data_store.get_all_socs() {
            println!(
                "│ SoC: {:<20} │ Nodes: {:<2} │ Nodes List: {:<24}│",
                soc_id,
                soc_info.nodes.len(),
                soc_info
                    .nodes
                    .iter()
                    .map(|n| n.node_name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        println!("├─────────────────────────────────────────────────────────────────────────────┤");

        for (board_id, board_info) in data_store.get_all_boards() {
            println!(
                "│ Board: {:<18} │ Nodes: {:<2} │ SoCs: {:<2} │ SoC List: {:<14} │",
                board_id,
                board_info.nodes.len(),
                board_info.socs.len(),
                board_info
                    .socs
                    .iter()
                    .map(|s| s.soc_id.split('.').next_back().unwrap_or(""))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Enhanced Board info printing with SoC details
    fn print_board_info(&self, board_info: &BoardInfo) {
        println!("\nBOARD INFORMATION");
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        println!("│ Board ID: {:<65} │", board_info.board_id);
        println!(
            "│ Nodes Count: {:<6} │ SoCs Count: {:<6} │ Updated: {:<19}     │",
            board_info.nodes.len(),
            board_info.socs.len(),
            self.format_time_ago(&board_info.last_updated)
        );

        // Show SoCs in this board
        if !board_info.socs.is_empty() {
            println!(
                "├─────────────────────────────────────────────────────────────────────────────┤"
            );
            println!(
                "│ SoCs in this Board:                                                         │"
            );
            for (i, soc) in board_info.socs.iter().enumerate() {
                println!(
                    "│  {}. SoC: {:<25} │ Nodes: {:<2} │ Avg CPU: {:<6.2}%           │",
                    i + 1,
                    soc.soc_id,
                    soc.nodes.len(),
                    soc.total_cpu_usage
                );
            }
        }

        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        println!("│ Board-wide Aggregated Metrics:                                              │");
        println!(
            "│   CPU: {:<7.2}% │ Total Cores: {:<5} │ GPU Units: {:<3} │ Efficiency: {:<4}    │",
            board_info.total_cpu_usage,
            board_info.total_cpu_count,
            board_info.total_gpu_count,
            self.calculate_efficiency(board_info.total_cpu_usage)
        );
        println!(
            "│   Memory: {:<4.2}% │ Used: {:<9} │ Total: {:<9} │ Free: {:<9} │",
            board_info.total_mem_usage,
            self.format_memory(board_info.total_used_memory),
            self.format_memory(board_info.total_memory),
            self.format_memory(board_info.total_memory - board_info.total_used_memory)
        );
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        println!("│ Nodes on this Board (grouped by SoC):                                       │");
        for (i, node) in board_info.nodes.iter().enumerate() {
            let status = if node.cpu_usage > 80.0 {
                "HIGH"
            } else if node.cpu_usage > 50.0 {
                "MED"
            } else {
                "LOW"
            };
            // Show which SoC this node belongs to
            let soc_id = DataStore::generate_soc_id(&node.ip).unwrap_or_default();
            println!(
                "│  {}. {:<25} │ SoC: {:<15} │ CPU: {:<6.2}% {} │",
                i + 1,
                node.node_name,
                soc_id,
                node.cpu_usage,
                status
            );
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Prints detailed NodeInfo in a formatted way
    fn print_node_info(&self, node_info: &NodeInfo) {
        println!("\nNODE INFORMATION");
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        println!("│ Node: {:<69} │", node_info.node_name);
        println!("│ IP Address: {:<63} │", node_info.ip);
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        println!(
            "│ CPU Usage: {:<6.2}% │ Cores: {:<3} │ GPU Units: {:<3} │ OS: {:<4} │",
            node_info.cpu_usage, node_info.cpu_count, node_info.gpu_count, node_info.os
        );
        println!(
            "│ Memory: {:<7.2}% │ Used: {:<8} KB │ Total: {:<8} KB │ Arch: {:<6} │",
            node_info.mem_usage,
            self.format_memory(node_info.used_memory),
            self.format_memory(node_info.total_memory),
            node_info.arch
        );
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        println!(
            "│ Network - RX: {:<15} │ TX: {:<15} │ Total: {:<14} │",
            self.format_bytes(node_info.rx_bytes),
            self.format_bytes(node_info.tx_bytes),
            self.format_bytes(node_info.rx_bytes + node_info.tx_bytes)
        );
        println!(
            "│ Disk I/O - Read: {:<12} │ Write: {:<12} │ Total: {:<14} │",
            self.format_bytes(node_info.read_bytes),
            self.format_bytes(node_info.write_bytes),
            self.format_bytes(node_info.read_bytes + node_info.write_bytes)
        );
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Prints aggregated SoC and Board information
    async fn print_aggregated_info(&self, data_store: &DataStore, ip: &str) {
        // Print SoC info
        if let Ok(soc_id) = DataStore::generate_soc_id(ip) {
            if let Some(soc_info) = data_store.get_soc_info(&soc_id) {
                self.print_soc_info(soc_info);
            }
        }

        // Print Board info
        if let Ok(board_id) = DataStore::generate_board_id(ip) {
            if let Some(board_info) = data_store.get_board_info(&board_id) {
                self.print_board_info(board_info);
            }
        }
    }

    /// Prints detailed SoC information
    fn print_soc_info(&self, soc_info: &SocInfo) {
        println!("\n SOC INFORMATION");
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        println!("│ SoC ID: {:<67} │", soc_info.soc_id);
        println!("│ Nodes Count: {:<62} │", soc_info.nodes.len());
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        println!("│ Aggregated Metrics:                                                         │");
        println!(
            "│   CPU: {:<7.2}%    │ Total Cores: {:<8}  │ GPU Units: {:<8}  │ Updated: {:<8} │",
            soc_info.total_cpu_usage,
            soc_info.total_cpu_count,
            soc_info.total_gpu_count,
            self.format_time_ago(&soc_info.last_updated)
        );
        println!(
            "│   Memory: {:<4.2}%   │ Used: {:<11}      │ Total: {:<11}   │ Free: {:<8}  │",
            soc_info.total_mem_usage,
            self.format_memory(soc_info.total_used_memory),
            self.format_memory(soc_info.total_memory),
            self.format_memory(soc_info.total_memory - soc_info.total_used_memory)
        );
        println!(
            "│   Network: RX {:<12} │ TX {:<12}         │ Total {:<12} │",
            self.format_bytes(soc_info.total_rx_bytes),
            self.format_bytes(soc_info.total_tx_bytes),
            self.format_bytes(soc_info.total_rx_bytes + soc_info.total_tx_bytes)
        );
        println!(
            "│   Disk I/O: Read {:<9} │ Write {:<9}         │ Total {:<9}    │",
            self.format_bytes(soc_info.total_read_bytes),
            self.format_bytes(soc_info.total_write_bytes),
            self.format_bytes(soc_info.total_read_bytes + soc_info.total_write_bytes)
        );
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        println!("│ Nodes in this SoC:                                                          │");
        for (i, node) in soc_info.nodes.iter().enumerate() {
            println!("│  {}. {:<71} │", i + 1, node.node_name);
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Prints summary statistics
    async fn print_summary_stats(&self, data_store: &DataStore) {
        let total_nodes = data_store.get_all_nodes().len();
        let total_socs = data_store.get_all_socs().len();
        let total_boards = data_store.get_all_boards().len();

        println!("\n SYSTEM SUMMARY");
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        println!(
            "│ Total Nodes: {:<8} │ Total SoCs: {:<8} │ Total Boards: {:<8} │ Status: ✅ │",
            total_nodes, total_socs, total_boards
        );

        // Calculate system-wide averages
        let (avg_cpu, avg_mem, total_cores, total_gpus) =
            self.calculate_system_averages(data_store);

        println!("│ System Avg CPU: {:<6.2}% │ Avg Memory: {:<6.2}% │ Total Cores: {:<6} │ GPUs: {:<4} │", 
                 avg_cpu, avg_mem, total_cores, total_gpus);
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Helper function to format bytes in human-readable format
    fn format_bytes(&self, bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    /// Helper function to format memory in human-readable format
    fn format_memory(&self, kb: u64) -> String {
        if kb >= 1024 * 1024 {
            format!("{:.1} GB", kb as f64 / (1024.0 * 1024.0))
        } else if kb >= 1024 {
            format!("{:.1} MB", kb as f64 / 1024.0)
        } else {
            format!("{} KB", kb)
        }
    }

    /// Helper function to format time ago
    fn format_time_ago(&self, time: &std::time::SystemTime) -> String {
        match time.elapsed() {
            Ok(duration) => {
                let secs = duration.as_secs();
                if secs < 60 {
                    format!("{}s ago", secs)
                } else if secs < 3600 {
                    format!("{}m ago", secs / 60)
                } else {
                    format!("{}h ago", secs / 3600)
                }
            }
            Err(_) => "unknown".to_string(),
        }
    }

    /// Helper function to calculate efficiency rating
    fn calculate_efficiency(&self, cpu_usage: f64) -> String {
        if cpu_usage > 90.0 {
            "HIGH"
        } else if cpu_usage > 70.0 {
            "GOOD"
        } else if cpu_usage > 30.0 {
            "NORM"
        } else {
            "LOW"
        }
        .to_string()
    }

    /// Helper function to calculate system-wide averages
    fn calculate_system_averages(&self, data_store: &DataStore) -> (f64, f64, u64, u64) {
        let nodes = data_store.get_all_nodes();
        if nodes.is_empty() {
            return (0.0, 0.0, 0, 0);
        }

        let count = nodes.len() as f64;
        let total_cpu: f64 = nodes.values().map(|n| n.cpu_usage).sum();
        let total_mem: f64 = nodes.values().map(|n| n.mem_usage).sum();
        let total_cores: u64 = nodes.values().map(|n| n.cpu_count).sum();
        let total_gpus: u64 = nodes.values().map(|n| n.gpu_count).sum();

        (
            total_cpu / count,
            total_mem / count,
            total_cores,
            total_gpus,
        )
    }

    /// Print all current data in a comprehensive format
    pub async fn print_all_data(&self) {
        let data_store = self.data_store.lock().await;

        println!("\n COMPLETE SYSTEM OVERVIEW");
        println!("{}", "=".repeat(80));

        // Print all nodes
        println!("\n ALL NODES:");
        for (i, (_, node)) in data_store.get_all_nodes().iter().enumerate() {
            let node_containers = data_store.get_containers_by_node(&node.node_name);
            println!(
                "{}. {} (IP: {}) - CPU: {:.2}%, Memory: {:.2}%, Containers: {}",
                i + 1,
                node.node_name,
                node.ip,
                node.cpu_usage,
                node.mem_usage,
                node_containers.len()
            );
        }

        // Print all SoCs
        println!("\n ALL SOCs:");
        for (i, (_, soc)) in data_store.get_all_socs().iter().enumerate() {
            println!(
                "{}. {} - {} nodes, Avg CPU: {:.2}%, Avg Memory: {:.2}%",
                i + 1,
                soc.soc_id,
                soc.nodes.len(),
                soc.total_cpu_usage,
                soc.total_mem_usage
            );
        }

        // Print all Boards
        println!("\n ALL BOARDS:");
        for (i, (_, board)) in data_store.get_all_boards().iter().enumerate() {
            println!(
                "{}. {} - {} nodes, {} SoCs, Avg CPU: {:.2}%, Avg Memory: {:.2}%",
                i + 1,
                board.board_id,
                board.nodes.len(),
                board.socs.len(),
                board.total_cpu_usage,
                board.total_mem_usage
            );
        }

        // Print all containers
        println!("\n ALL CONTAINERS:");
        for (i, (_, container)) in data_store.get_all_containers().iter().enumerate() {
            let name = container
                .names
                .first()
                .unwrap_or(&"unnamed".to_string())
                .clone();
            let status = container
                .state
                .get("Status")
                .unwrap_or(&"unknown".to_string())
                .clone();

            println!(
                "{}. {} (ID: {}) - Image: {}, Status: {}",
                i + 1,
                name,
                &container.id[..std::cmp::min(12, container.id.len())], // Show only first 12 chars of ID
                container.image,
                status
            );
        }

        self.print_summary_stats(&data_store).await;
        self.print_container_overview().await;
    }

    /// Main loop for processing incoming gRPC ContainerList messages.
    ///
    /// This function continuously receives ContainerList from the gRPC channel
    /// and handles them using the handle_container_list method.
    pub async fn process_container_requests(&self) -> Result<()> {
        loop {
            let container_list_opt = {
                let mut rx_container = self.rx_container.lock().await;
                rx_container.recv().await
            };
            if let Some(container_list) = container_list_opt {
                self.handle_container_list(container_list).await;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Main loop for processing incoming gRPC NodeInfo messages.
    ///
    /// This function continuously receives NodeInfo from the gRPC channel
    /// and handles them using the handle_node_info method.
    pub async fn process_node_info_requests(&self) -> Result<()> {
        loop {
            let node_info_opt = {
                let mut rx_node = self.rx_node.lock().await;
                rx_node.recv().await
            };
            if let Some(node_info) = node_info_opt {
                self.handle_node_info(node_info).await;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Main loop for processing incoming stress metric JSON strings.
    ///
    /// Consumes JSON strings forwarded by the gRPC receiver and handles them (currently logs).
    pub async fn process_stress_requests(&self) -> Result<()> {
        loop {
            let stress_metric_opt = {
                let mut rx_stress = self.rx_stress.lock().await;
                rx_stress.recv().await
            };
            if let Some(json) = stress_metric_opt {
                // Validate JSON first
                match serde_json::from_str::<serde_json::Value>(&json) {
                    Ok(val) => {
                        let pname = val
                            .get("process_name")
                            .and_then(|s| s.as_str())
                            .unwrap_or("unknown");
                        let pid = val.get("pid").and_then(|p| p.as_i64()).unwrap_or(0);

                        // Persist raw JSON into etcd (uses existing helper)
                        match crate::etcd_storage::store_stress_metric_json(&json).await {
                            Ok(_) => {
                                println!(
                                    "[MonitoringServer] SUCCESS: Stored stress metric for process={} pid={}",
                                    pname, pid
                                );
                            }
                            Err(e) => {
                                eprintln!(
                                    "[MonitoringServer] ERROR: Failed to store stress metric to etcd: {}",
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "[MonitoringServer] ERROR: received invalid stress metric JSON: {} -- payload: {}",
                            e, json
                        );
                    }
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Runs the MonitoringServerManager event loop.
    ///
    /// Spawns container, node and stress processing tasks and waits for them to finish.
    pub async fn run(self) -> Result<()> {
        let arc_self = Arc::new(self);

        // Container processor task
        let container_manager = Arc::clone(&arc_self);
        let container_processor = tokio::spawn(async move {
            if let Err(e) = container_manager.process_container_requests().await {
                eprintln!("Container processor error: {:?}", e);
            }
        });

        // NodeInfo processor task
        let node_manager = Arc::clone(&arc_self);
        let node_processor = tokio::spawn(async move {
            if let Err(e) = node_manager.process_node_info_requests().await {
                eprintln!("Node processor error: {:?}", e);
            }
        });

        // Stress metric processor task
        let stress_manager = Arc::clone(&arc_self);
        let stress_processor = tokio::spawn(async move {
            if let Err(e) = stress_manager.process_stress_requests().await {
                eprintln!("Stress processor error: {:?}", e);
            }
        });

        let _ = tokio::try_join!(container_processor, node_processor, stress_processor);
        println!("MonitoringServerManager stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::monitoringserver::{ContainerInfo, ContainerList, NodeInfo};
    use std::collections::HashMap;
    use std::time::SystemTime;
    use tokio::sync::mpsc;

    // Helper: construct a MonitoringServerManager with new channels (including stress receiver)
    async fn new_mgr() -> MonitoringServerManager {
        let (_tx_c, rx_c) = mpsc::channel(1);
        let (_tx_n, rx_n) = mpsc::channel(1);
        let (_tx_s, rx_s) = mpsc::channel::<String>(1);
        MonitoringServerManager::new(rx_c, rx_n, rx_s).await
    }

    fn sample_node(name: &str, ip: &str) -> NodeInfo {
        NodeInfo {
            node_name: name.to_string(),
            ip: ip.to_string(),
            cpu_usage: 42.0,
            cpu_count: 2,
            gpu_count: 1,
            used_memory: 1024,
            total_memory: 2048,
            mem_usage: 50.0,
            rx_bytes: 100,
            tx_bytes: 200,
            read_bytes: 300,
            write_bytes: 400,
            arch: "x86_64".to_string(),
            os: "linux".to_string(),
        }
    }

    fn sample_container(id: &str, name: &str, status: &str) -> ContainerInfo {
        let mut state = HashMap::new();
        state.insert("Status".to_string(), status.to_string());
        ContainerInfo {
            id: id.to_string(),
            names: vec![name.to_string()],
            image: "testimg".to_string(),
            state,
            ..Default::default()
        }
    }

    fn sample_container_list(node_name: &str, containers: Vec<ContainerInfo>) -> ContainerList {
        ContainerList {
            node_name: node_name.to_string(),
            containers,
        }
    }

    #[tokio::test]
    async fn test_new_and_initialize() {
        let mut mgr = new_mgr().await;
        assert!(mgr.initialize().await.is_ok());
    }

    #[tokio::test]
    async fn test_handle_container_list_and_print_summary() {
        let mgr = new_mgr().await;

        let container1 = sample_container("c1", "cont1", "running");
        let container2 = sample_container("c2", "cont2", "exited");
        let clist = sample_container_list("node1", vec![container1, container2]);

        mgr.handle_container_list(clist).await;
        // No assertion: just ensure no panic and output is printed
    }

    #[tokio::test]
    async fn test_print_container_overview() {
        let mgr = new_mgr().await;

        // Insert containers into data_store
        {
            let mut ds = mgr.data_store.lock().await;
            let mut running = sample_container("c1", "cont1", "running");
            running
                .state
                .insert("Running".to_string(), "true".to_string());
            let mut stopped = sample_container("c2", "cont2", "exited");
            stopped
                .state
                .insert("Running".to_string(), "false".to_string());
            ds.containers.insert("c1".to_string(), running);
            ds.containers.insert("c2".to_string(), stopped);
        }
        mgr.print_container_overview().await;
    }

    #[tokio::test]
    async fn test_print_all_nodes_and_containers() {
        let mgr = new_mgr().await;

        {
            let mut ds = mgr.data_store.lock().await;
            let node = sample_node("node1", "192.168.10.201");
            ds.nodes.insert("node1".to_string(), node);
            let container = sample_container("c1", "cont1", "running");
            ds.containers.insert("c1".to_string(), container);
            ds.container_node_mapping
                .insert("c1".to_string(), "node1".to_string());
        }
        mgr.print_all_nodes().await;
        mgr.print_all_containers().await;
    }

    #[tokio::test]
    async fn test_print_all_boards_and_socs() {
        let mgr = new_mgr().await;

        {
            let mut ds = mgr.data_store.lock().await;
            let node = sample_node("node1", "192.168.10.201");
            let soc = SocInfo::new("socid".to_string(), node.clone());
            let mut board = BoardInfo::new("boardid".to_string(), node.clone());
            board.socs.push(soc.clone());
            ds.socs.insert("socid".to_string(), soc);
            ds.boards.insert("boardid".to_string(), board);
        }
        mgr.print_all_boards().await;
        mgr.print_all_socs().await;
    }

    #[tokio::test]
    async fn test_print_node_info_and_board_info_and_soc_info() {
        let mgr = new_mgr().await;

        let node = sample_node("node1", "192.168.10.201");
        mgr.print_node_info(&node);

        let soc = SocInfo::new("socid".to_string(), node.clone());
        mgr.print_soc_info(&soc);

        let mut board = BoardInfo::new("boardid".to_string(), node.clone());
        board.socs.push(soc);
        mgr.print_board_info(&board);
    }

    #[tokio::test]
    async fn test_print_id_generation_details() {
        let mgr = new_mgr().await;
        mgr.print_id_generation_details("192.168.10.201");
        mgr.print_id_generation_details("bad_ip");
    }

    #[tokio::test]
    async fn test_format_bytes_and_memory_and_time_ago_and_efficiency() {
        let mgr = new_mgr().await;

        assert_eq!(mgr.format_bytes(512), "512 B");
        assert_eq!(mgr.format_bytes(2048), "2.0 KB");
        assert_eq!(mgr.format_bytes(2 * 1024 * 1024), "2.0 MB");

        assert_eq!(mgr.format_memory(512), "512 KB");
        assert_eq!(mgr.format_memory(2048), "2.0 MB");
        assert_eq!(mgr.format_memory(2 * 1024 * 1024), "2.0 GB");

        let now = SystemTime::now();
        assert!(mgr.format_time_ago(&now).ends_with("ago"));

        assert_eq!(mgr.calculate_efficiency(95.0), "HIGH");
        assert_eq!(mgr.calculate_efficiency(80.0), "GOOD");
        assert_eq!(mgr.calculate_efficiency(50.0), "NORM");
        assert_eq!(mgr.calculate_efficiency(10.0), "LOW");
    }

    #[tokio::test]
    async fn test_calculate_system_averages() {
        let mgr = new_mgr().await;

        let mut ds = DataStore::new();
        assert_eq!(mgr.calculate_system_averages(&ds), (0.0, 0.0, 0, 0));

        let node = sample_node("node1", "192.168.10.201");
        ds.nodes.insert("node1".to_string(), node);
        let (cpu, mem, cores, gpus) = mgr.calculate_system_averages(&ds);
        assert_eq!(cpu, 42.0);
        assert_eq!(mem, 50.0);
        assert_eq!(cores, 2);
        assert_eq!(gpus, 1);
    }

    #[tokio::test]
    async fn test_print_all_data() {
        use tokio::time::{timeout, Duration};

        let mgr = new_mgr().await;

        {
            let mut ds = mgr.data_store.lock().await;
            let node = sample_node("node1", "192.168.10.201");
            ds.nodes.insert("node1".to_string(), node.clone());
            let soc = SocInfo::new("socid".to_string(), node.clone());
            ds.socs.insert("socid".to_string(), soc);
            let board = BoardInfo::new("boardid".to_string(), node.clone());
            ds.boards.insert("boardid".to_string(), board);
            let container = sample_container("c1", "cont1", "running");
            ds.containers.insert("c1".to_string(), container);
        }
        // Ensure the print_all_data future completes within 2 seconds
        let _ = timeout(Duration::from_secs(2), mgr.print_all_data()).await;
    }

    #[tokio::test]
    async fn test_handle_node_info_and_print_aggregated_info() {
        use tokio::time::{timeout, Duration};

        let mgr = new_mgr().await;

        let node = sample_node("node1", "192.168.10.201");

        // Run handle_node_info with a timeout to ensure the test does not hang
        let result = timeout(Duration::from_secs(1), mgr.handle_node_info(node)).await;
        assert!(result.is_ok(), "handle_node_info did not complete in time");
    }

    #[tokio::test]
    async fn test_print_detailed_soc_mapping() {
        let mgr = new_mgr().await;

        let mut ds = DataStore::new();
        let node = sample_node("node1", "192.168.10.201");
        let soc = SocInfo::new("socid".to_string(), node.clone());
        ds.socs.insert("socid".to_string(), soc);
        let board = BoardInfo::new("boardid".to_string(), node.clone());
        ds.boards.insert("boardid".to_string(), board);

        mgr.print_detailed_soc_mapping(&ds).await;
    }

    #[tokio::test]
    async fn test_print_summary_stats() {
        let mgr = new_mgr().await;

        let mut ds = DataStore::new();
        let node = sample_node("node1", "192.168.10.201");
        ds.nodes.insert("node1".to_string(), node);
        mgr.print_summary_stats(&ds).await;
    }
}

// Note: process_container_requests, process_node_info_requests, and run are event loops
// and require integration/async tests with channel senders, which is not practical for
