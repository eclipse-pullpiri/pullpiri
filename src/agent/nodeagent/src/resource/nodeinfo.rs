use super::NodeInfo;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use sysinfo::{Networks, System};

// Static storage for previous IO/network values for delta calculation
static PREV_IO: Lazy<Mutex<Option<(u64, u64, u64, u64)>>> = Lazy::new(|| Mutex::new(None));

/// Returns NodeInfo with rx_bytes, tx_bytes, read_bytes, write_bytes as deltas since last call.
pub fn extract_node_info_delta() -> NodeInfo {
    let mut sys = System::new_all();
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_cpu_usage();

    // 1. CPU
    let cpu_count = sys.cpus().len();
    // Average all logical CPU usage values
    let cpu_usage = if cpu_count > 0 {
        sys.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpu_count as f32
    } else {
        0.0
    };

    // 2. GPU
    let gpu_count = match std::fs::read_dir("/sys/class/drm/") {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let fname = entry.file_name();
                let fname = fname.to_string_lossy();
                fname.starts_with("card") && fname.chars().skip(4).all(|c| c.is_ascii_digit())
            })
            .count(),
        Err(_) => 0,
    };

    // 3. Memory
    let total_memory = sys.total_memory();
    let used_memory = sys.used_memory();
    let mem_usage = if total_memory > 0 {
        (used_memory as f32) / (total_memory as f32) * 100.0
    } else {
        0.0
    };

    // 4. Network (sum over all interfaces)
    let networks = Networks::new_with_refreshed_list();
    // summation of each interface's received and transmitted bytes
    let mut rx_bytes_now: u64 = 0;
    let mut tx_bytes_now: u64 = 0;
    for (_, data) in &networks {
        rx_bytes_now += data.total_received();
        tx_bytes_now += data.total_transmitted();
    }

    // 5. Storage (read_bytes, write_bytes) from /proc/diskstats
    let (read_bytes_now, write_bytes_now) = get_disk_io_bytes();

    // Calculate deltas from previous values
    let mut prev = PREV_IO.lock().unwrap();
    let (rx_bytes, tx_bytes, read_bytes, write_bytes) =
        if let Some((prev_rx, prev_tx, prev_read, prev_write)) = *prev {
            (
                rx_bytes_now.saturating_sub(prev_rx),
                tx_bytes_now.saturating_sub(prev_tx),
                read_bytes_now.saturating_sub(prev_read),
                write_bytes_now.saturating_sub(prev_write),
            )
        } else {
            (0, 0, 0, 0)
        };
    // Store current values for next delta calculation
    *prev = Some((rx_bytes_now, tx_bytes_now, read_bytes_now, write_bytes_now));

    // OS and Arch extraction using sysinfo
    let os = sysinfo::System::long_os_version().unwrap_or_else(|| "Unknown".to_string());
    let arch = sysinfo::System::cpu_arch();

    // IP extraction (first non-loopback IPv4)
    let ip = get_local_ip().unwrap_or_else(|| "Unknown".to_string());

    NodeInfo {
        cpu_count,
        cpu_usage,
        gpu_count,
        total_memory,
        used_memory,
        mem_usage,
        rx_bytes,
        tx_bytes,
        read_bytes,
        write_bytes,
        os,
        arch,
        ip,
    }
}

/// Returns the first non-loopback IPv4 address as a String, or None if not found.
fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;
    // This does not actually send a packet
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(local_addr) = socket.local_addr() {
                let ip = local_addr.ip();
                if ip.is_ipv4() && !ip.is_loopback() {
                    return Some(ip.to_string());
                }
            }
        }
    }
    None
}

/// Returns (read_bytes, write_bytes) by parsing /proc/diskstats and summing all block devices.
fn get_disk_io_bytes() -> (u64, u64) {
    let mut read_sectors = 0u64;
    let mut write_sectors = 0u64;
    if let Ok(content) = std::fs::read_to_string("/proc/diskstats") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 13 {
                let name = parts[2];
                // Only count physical disks (skip loop, ram, partitions, etc.)
                if name.starts_with("sd")
                    || name.starts_with("hd")
                    || name.starts_with("vd")
                    || name.starts_with("nvme")
                {
                    // Field 5: sectors read, Field 9: sectors written
                    read_sectors += parts[5].parse::<u64>().unwrap_or(0);
                    write_sectors += parts[9].parse::<u64>().unwrap_or(0);
                }
            }
        }
    }
    // Most systems use 512 bytes per sector
    (read_sectors * 512, write_sectors * 512)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_node_info_delta() {
        let info = extract_node_info_delta();
        assert!(info.cpu_usage >= 0.0 && info.cpu_usage <= 100.0);
        assert!(info.cpu_count > 0);
        assert!(info.total_memory > 0);
        assert!(info.used_memory <= info.total_memory);
        assert!(info.mem_usage >= 0.0 && info.mem_usage <= 100.0);
        // Removed always-true u64 >= 0 assertions
    }
}
