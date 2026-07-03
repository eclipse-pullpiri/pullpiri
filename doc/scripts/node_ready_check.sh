#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0
# node_ready_check.sh - Node clustering pre-state check script
# Optimized system check for embedded environments

# Exit on error
set -e

# Check for root privileges
if [ "$(id -u)" -ne 0 ]; then
    echo "Warning: This script should ideally be run as root for complete system checks."
    # Continue anyway as we might be running in a limited-privilege environment
fi

NODE_TYPE=${1:-"sub"}  # Default is "sub" node
LOG_FILE="/var/log/pullpiri/system_check.log"
RESULT_FILE="/var/run/pullpiri/node_status"

# Create log directories
mkdir -p $(dirname $LOG_FILE) $(dirname $RESULT_FILE)

# Initialize counters
ERROR_COUNT=0
WARNING_COUNT=0

# Log function
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a $LOG_FILE
}

log "Starting system readiness check (Node type: $NODE_TYPE)"

# Initialize results
echo "status=checking" > $RESULT_FILE

# Check if command exists
command_exists() {
    command -v "$1" &> /dev/null
}

# 1. Check basic system resources
log "Checking basic system resources..."

# Check CPU load
if [ -f /proc/loadavg ]; then
    CPU_LOAD=$(cat /proc/loadavg 2>/dev/null | awk '{print $1}' || echo "unknown")
    
    if [ "$CPU_LOAD" != "unknown" ] && [ -f /proc/cpuinfo ]; then
        CPU_CORES=$(grep -c ^processor /proc/cpuinfo 2>/dev/null || echo 1)
        
        # Check if bc is available
        if command_exists bc; then
            CPU_LOAD_PER_CORE=$(echo "$CPU_LOAD / $CPU_CORES" | bc -l 2>/dev/null || echo "unknown")
            
            if [ "$CPU_LOAD_PER_CORE" != "unknown" ]; then
                if (( $(echo "$CPU_LOAD_PER_CORE > 0.8" | bc -l 2>/dev/null || echo 0) )); then
                    log "Warning: High CPU load: $CPU_LOAD ($(printf "%.2f" $CPU_LOAD_PER_CORE) per core)"
                    WARNING_COUNT=$((WARNING_COUNT+1))
                else
                    log "CPU load normal: $CPU_LOAD ($(printf "%.2f" $CPU_LOAD_PER_CORE) per core)"
                fi
            else
                log "Warning: Could not calculate CPU load per core"
                WARNING_COUNT=$((WARNING_COUNT+1))
            fi
        else
            log "Warning: 'bc' command not available, skipping detailed CPU load check"
            WARNING_COUNT=$((WARNING_COUNT+1))
            log "CPU load: $CPU_LOAD (total)"
        fi
    else
        log "Warning: Could not determine CPU load or core count"
        WARNING_COUNT=$((WARNING_COUNT+1))
    fi
else
    log "Warning: /proc/loadavg not found, skipping CPU load check"
    WARNING_COUNT=$((WARNING_COUNT+1))
fi

# Check memory
if [ -f /proc/meminfo ]; then
    MEM_TOTAL=$(grep MemTotal /proc/meminfo 2>/dev/null | awk '{print $2}' || echo 0)
    MEM_FREE=$(grep MemAvailable /proc/meminfo 2>/dev/null | awk '{print $2}' || echo 0)
    
    if [ "$MEM_TOTAL" -gt 0 ] && [ "$MEM_FREE" -gt 0 ]; then
        # Check if bc is available
        if command_exists bc; then
            MEM_PERCENT_FREE=$(echo "scale=2; $MEM_FREE * 100 / $MEM_TOTAL" | bc 2>/dev/null || echo 0)
            
            if (( $(echo "$MEM_PERCENT_FREE < 20" | bc -l 2>/dev/null || echo 0) )); then
                log "Warning: Low available memory: ${MEM_PERCENT_FREE}% remaining"
                WARNING_COUNT=$((WARNING_COUNT+1))
            else
                log "Memory status normal: ${MEM_PERCENT_FREE}% available"
            fi
        else
            # Fallback to basic integer math
            MEM_PERCENT_FREE=$((MEM_FREE * 100 / MEM_TOTAL))
            
            if [ "$MEM_PERCENT_FREE" -lt 20 ]; then
                log "Warning: Low available memory: ${MEM_PERCENT_FREE}% remaining"
                WARNING_COUNT=$((WARNING_COUNT+1))
            else
                log "Memory status normal: ${MEM_PERCENT_FREE}% available"
            fi
        fi
    else
        log "Warning: Could not determine memory usage"
        WARNING_COUNT=$((WARNING_COUNT+1))
    fi
else
    log "Warning: /proc/meminfo not found, skipping memory check"
    WARNING_COUNT=$((WARNING_COUNT+1))
fi

# Check disk space
DISK_SPACE_CHECK=false
if command_exists df; then
    ROOT_USAGE=$(df -h / | awk 'NR==2 {print $5}' | sed 's/%//' 2>/dev/null || echo "unknown")
    
    if [ "$ROOT_USAGE" != "unknown" ]; then
        DISK_SPACE_CHECK=true
        if [ "$ROOT_USAGE" -gt 90 ]; then
            log "Error: Root filesystem is almost full: ${ROOT_USAGE}% used"
            ERROR_COUNT=$((ERROR_COUNT+1))
        elif [ "$ROOT_USAGE" -gt 80 ]; then
            log "Warning: Root filesystem usage is high: ${ROOT_USAGE}% used"
            WARNING_COUNT=$((WARNING_COUNT+1))
        else
            log "Disk space normal: ${ROOT_USAGE}% used on root filesystem"
        fi
    fi
fi

if [ "$DISK_SPACE_CHECK" = false ]; then
    log "Warning: Could not check disk space"
    WARNING_COUNT=$((WARNING_COUNT+1))
fi

# 2. Check essential services
log "Checking essential services..."

# Check Podman
if command_exists podman; then
    PODMAN_VERSION=$(podman --version | awk '{print $3}' 2>/dev/null || echo "unknown")
    log "Podman installed: version $PODMAN_VERSION"
    
    # Check Podman service
    if command_exists systemctl; then
        if ! systemctl is-active --quiet podman.socket 2>/dev/null; then
            log "Warning: podman.socket service is not running."
            WARNING_COUNT=$((WARNING_COUNT+1))
        else
            log "podman.socket service running"
        fi
    else
        log "Warning: systemctl not available, skipping podman.socket service check"
        WARNING_COUNT=$((WARNING_COUNT+1))
    fi
else
    log "Error: Podman is not installed."
    ERROR_COUNT=$((ERROR_COUNT+1))
fi

# 3. Check network connectivity
log "Checking network connectivity..."

# Check if master node IP is set
MASTER_IP=${MASTER_NODE_IP:-"127.0.0.1"}
#GRPC_PORT=${GRPC_PORT:-47098}
GRPC_PORT="47098"

log "Using master node IP: $MASTER_IP (from environment or default)"
log "Using gRPC port: $GRPC_PORT (from environment or default)"

# Check ping command
if command_exists ping; then
    if ping -c 1 -W 2 $MASTER_IP &> /dev/null; then
        log "Master node is reachable: $MASTER_IP"
        
        # Check netcat command
        if command_exists nc; then
            # Check API server port
            if nc -z -w 2 $MASTER_IP $GRPC_PORT &> /dev/null; then
                log "API server gRPC port accessible: $MASTER_IP:$GRPC_PORT"
            else
                log "Error: Cannot connect to API server gRPC port: $MASTER_IP:$GRPC_PORT"
                ERROR_COUNT=$((ERROR_COUNT+1))
            fi
            
            # Check RocksDB port
            if nc -z -w 2 $MASTER_IP 47007 &> /dev/null; then
                log "RocksDB port accessible: $MASTER_IP:47007"
            else
                log "Error: Cannot connect to RocksDB port: $MASTER_IP:47007"
                ERROR_COUNT=$((ERROR_COUNT+1))
            fi
        else
            log "Warning: 'nc' command not available, skipping port connectivity checks"
            WARNING_COUNT=$((WARNING_COUNT+1))
        fi
    else
        log "Error: Cannot reach master node: $MASTER_IP"
        ERROR_COUNT=$((ERROR_COUNT+1))
    fi
else
    log "Warning: 'ping' command not available, skipping connectivity checks"
    WARNING_COUNT=$((WARNING_COUNT+1))
fi

# Check system clock
if command_exists date; then
    log "Current system time: $(date)"
    
    # Check if ntp or chrony is running
    if command_exists systemctl; then
        if systemctl is-active --quiet systemd-timesyncd 2>/dev/null || 
           systemctl is-active --quiet ntpd 2>/dev/null ||
           systemctl is-active --quiet chronyd 2>/dev/null; then
            log "Time synchronization service is running"
        else
            log "Warning: No time synchronization service detected"
            WARNING_COUNT=$((WARNING_COUNT+1))
        fi
    fi
else
    log "Warning: Could not check system time"
    WARNING_COUNT=$((WARNING_COUNT+1))
fi

# 4. Evaluate status and output results
log "System check completed: Errors($ERROR_COUNT), Warnings($WARNING_COUNT)"

if [ $ERROR_COUNT -gt 0 ]; then
    log "System readiness status: Failed (Critical errors occurred)"
    echo "status=failed" > $RESULT_FILE
    exit 1
elif [ $WARNING_COUNT -gt 0 ]; then
    log "System readiness status: Warning (Non-critical issues found)"
    echo "status=warning" > $RESULT_FILE
    exit 0
else
    log "System readiness status: Good (All checks passed)"
    echo "status=ready" > $RESULT_FILE
    exit 0
fi
