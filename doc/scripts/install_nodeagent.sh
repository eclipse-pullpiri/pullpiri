#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0
# install_nodeagent.sh - NodeAgent installation script
# Usage: ./install_nodeagent.sh --master <master_ip> --node <node_ip> [--role <node_role>] [--type <node_type>]

# Exit on error
set -e

# Check root privileges
if [ "$(id -u)" -ne 0 ]; then
    echo "Error: This script must be run as root."
    exit 1
fi

# Initialize variables
MASTER_IP="127.0.0.1" # Board IP, Bluechi controller
NODE_IP="127.0.0.1"   # AWS EC2 IP is 10.0.0.20, if device is board, change NODE_IP
NODE_NAME=$(hostname)  # Always use system hostname
NODE_ROLE="nodeagent"  # Default node role (master, nodeagent, bluechi)
NODE_TYPE="vehicle"  # Default node type (vehicle, cloud)

# Process command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --master)
            MASTER_IP="$2"
            shift 2
            ;;
        --node)
            NODE_IP="$2"
            shift 2
            ;;
        --role)
            NODE_ROLE="$2"
            # Validate node role
            if [[ ! "$NODE_ROLE" =~ ^(master|nodeagent|bluechi)$ ]]; then
                echo "Error: Invalid node role '$NODE_ROLE'. Must be one of: master, nodeagent, bluechi"
                exit 1
            fi
            shift 2
            ;;
        --type)
            NODE_TYPE="$2"
            # Validate node type
            if [[ ! "$NODE_TYPE" =~ ^(vehicle|cloud|master|nodeagent|bluechi)$ ]]; then
                echo "Error: Invalid node type '$NODE_TYPE'. Must be one of: vehicle, cloud, master, nodeagent, bluechi"
                exit 1
            fi
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 --master <master_ip> --node <node_ip> [--role <node_role>] [--type <node_type>]"
            exit 1
            ;;
    esac
done

# Parameter validation
if [ -z "$MASTER_IP" ] || [ -z "$NODE_IP" ]; then
    echo "Error: Both --master and --node options are required."
    echo "Usage: $0 --master <master_ip> --node <node_ip> [--role <node_role>] [--type <node_type>]"
    exit 1
fi

# Initialize counters
ERROR_COUNT=0
WARNING_COUNT=0

# Function to handle errors
handle_error() {
    local exit_code=$?
    local line_number=$1
    if [ $exit_code -ne 0 ]; then
        echo "Error: Command failed at line $line_number with exit code $exit_code"
        ERROR_COUNT=$((ERROR_COUNT+1))
    fi
}
trap 'handle_error $LINENO' ERR

# Function to install required packages
install_required_packages() {
    echo "Checking required package installation..."
    
    # Detect package manager
    if command -v apt-get &> /dev/null; then
        PKG_MANAGER="apt-get"
        PKG_UPDATE="apt-get update"
        PKG_INSTALL="apt-get install -y"
    elif command -v dnf &> /dev/null; then
        PKG_MANAGER="dnf"
        PKG_UPDATE="dnf check-update || true"  # Ignore exit code 100
        PKG_INSTALL="dnf install -y"
    elif command -v yum &> /dev/null; then
        PKG_MANAGER="yum"
        PKG_UPDATE="yum check-update || true"  # Ignore exit code 100
        PKG_INSTALL="yum install -y"
    elif command -v zypper &> /dev/null; then
        PKG_MANAGER="zypper"
        PKG_UPDATE="zypper refresh"
        PKG_INSTALL="zypper install -y"
    elif command -v pacman &> /dev/null; then
        PKG_MANAGER="pacman"
        PKG_UPDATE="pacman -Sy"
        PKG_INSTALL="pacman -S --noconfirm"
    else
        echo "Warning: No supported package manager found. You may need to install required packages manually."
        WARNING_COUNT=$((WARNING_COUNT+1))
        return 1
    fi
    
    # Update package repository
    echo "Updating package repository..."
    eval $PKG_UPDATE || {
        echo "Warning: Failed to update package repository. Continuing anyway."
        WARNING_COUNT=$((WARNING_COUNT+1))
    }
    
    # List of required packages
    REQUIRED_PACKAGES=()
    
    # Check and add curl
    if ! command -v curl &> /dev/null; then
        REQUIRED_PACKAGES+=("curl")
    fi
    
    # Check and add wget
    if ! command -v wget &> /dev/null && ! command -v curl &> /dev/null; then
        REQUIRED_PACKAGES+=("wget")
    fi
    
    # Check and add netcat
    if ! command -v nc &> /dev/null; then
        if [ "$PKG_MANAGER" = "apt-get" ]; then
            REQUIRED_PACKAGES+=("netcat")
        else
            REQUIRED_PACKAGES+=("nc")
        fi
    fi
    
    # Check and add bc
    if ! command -v bc &> /dev/null; then
        REQUIRED_PACKAGES+=("bc")
    fi
    
    # Check and add podman
    if ! command -v podman &> /dev/null; then
        REQUIRED_PACKAGES+=("podman")
    fi
    
    # Install required packages
    if [ ${#REQUIRED_PACKAGES[@]} -gt 0 ]; then
        echo "Installing the following required packages: ${REQUIRED_PACKAGES[*]}"
        $PKG_INSTALL "${REQUIRED_PACKAGES[@]}" || {
            echo "Warning: Failed to install some required packages. Some features may not work correctly."
            WARNING_COUNT=$((WARNING_COUNT+1))
        }
    else
        echo "All required packages are already installed."
    fi
}

# Parameter settings already processed from command line
# MASTER_IP and NODE_IP are set from command line arguments
NODEAGENT_PORT="47004"
APISERVER_PORT="47098"
VERSION_TXT="./version.txt"
VERSION=$(cat "$VERSION_TXT")
DOWNLOAD_URL="https://github.com/eclipse-pullpiri/pullpiri/releases/download/${VERSION}"
CHECKSUM_URL="${DOWNLOAD_URL}"  # Define CHECKSUM_URL
INSTALL_DIR="/opt/pullpiri"
CONFIG_DIR="/etc/pullpiri"
BINARY_NAME="nodeagent"
LOG_DIR="/var/log/pullpiri"
DATA_DIR="/var/lib/pullpiri"
RUN_DIR="/var/run/pullpiri"
YAML_STORAGE_DIR="${CONFIG_DIR}/yaml"

# Install required packages
install_required_packages

# Create necessary directories
echo "Creating necessary directories..."
mkdir -p ${INSTALL_DIR} ${CONFIG_DIR} ${LOG_DIR} ${DATA_DIR} ${RUN_DIR} ${YAML_STORAGE_DIR}

# Download NodeAgent binary
echo "Downloading NodeAgent binary... (${DOWNLOAD_URL})"
ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then
    BINARY_SUFFIX="linux-amd64"
elif [ "$ARCH" = "aarch64" ]; then
    BINARY_SUFFIX="linux-arm64"
elif [[ "$ARCH" == "arm"* ]]; then
    BINARY_SUFFIX="linux-arm64"
else
    BINARY_SUFFIX="$ARCH"
fi

DOWNLOAD_SUCCESS=false
if command -v curl &> /dev/null; then
    if curl -fL "${DOWNLOAD_URL}/nodeagent-${BINARY_SUFFIX}" \
        -o "${INSTALL_DIR}/${BINARY_NAME}"; then
        DOWNLOAD_SUCCESS=true
    fi
elif command -v wget &> /dev/null; then
    if wget --content-disposition \
        "${DOWNLOAD_URL}/nodeagent-${BINARY_SUFFIX}" \
        -O "${INSTALL_DIR}/${BINARY_NAME}"; then
        DOWNLOAD_SUCCESS=true
    fi
else
    echo "Error: curl or wget is not installed."
    exit 1
fi

# Verify download
if [ "$DOWNLOAD_SUCCESS" = false ] || [ ! -f "${INSTALL_DIR}/${BINARY_NAME}" ]; then
    echo "Error: Failed to download NodeAgent binary."
    exit 1
fi

# Integrity check (optional)
if command -v sha256sum &> /dev/null; then
    echo "Verifying binary integrity..."
    if curl -L ${CHECKSUM_URL}/checksums.txt -o /tmp/pullpiri_checksums.txt --fail; then
        CHECKSUM_PATTERN="${BINARY_NAME}-${BINARY_SUFFIX}"
        if grep -q "$CHECKSUM_PATTERN" /tmp/pullpiri_checksums.txt; then
            if ! (cd ${INSTALL_DIR} && sha256sum -c <(grep "$CHECKSUM_PATTERN" /tmp/pullpiri_checksums.txt)); then
                echo "Warning: Binary checksum does not match. This may indicate a corrupted download."
                WARNING_COUNT=$((WARNING_COUNT+1))
            else
                echo "Binary integrity verified successfully."
            fi
        else
            echo "Warning: Could not find checksum for ${CHECKSUM_PATTERN} in checksums.txt"
            WARNING_COUNT=$((WARNING_COUNT+1))
        fi
        rm /tmp/pullpiri_checksums.txt
    else
        echo "Warning: Could not download checksum file. Skipping integrity check."
        WARNING_COUNT=$((WARNING_COUNT+1))
    fi
fi

# Grant execution permission
chmod +x ${INSTALL_DIR}/${BINARY_NAME}

# Download system check script
#echo "Downloading system check script..."
#SCRIPT_DOWNLOAD_SUCCESS=false
#if command -v curl &> /dev/null; then
#    if curl -L "${DOWNLOAD_URL}/node_ready_check.sh" -o /usr/local/bin/node_ready_check.sh --fail; then
#        SCRIPT_DOWNLOAD_SUCCESS=true
#    fi
#elif command -v wget &> /dev/null; then
#    if wget "${DOWNLOAD_URL}/node_ready_check.sh" -O /usr/local/bin/node_ready_check.sh; then
#        SCRIPT_DOWNLOAD_SUCCESS=true
#    fi
#fi
cp ./node_ready_check.sh /usr/local/bin/node_ready_check.sh
SCRIPT_DOWNLOAD_SUCCESS=true

# Verify download and create default if needed
if [ "$SCRIPT_DOWNLOAD_SUCCESS" = false ] || [ ! -f /usr/local/bin/node_ready_check.sh ]; then
    echo "Warning: Failed to download system check script. Creating default check script."
    WARNING_COUNT=$((WARNING_COUNT+1))
    cat > /usr/local/bin/node_ready_check.sh << 'EOF'
#!/bin/bash
# Default system check script
echo "Performing default system check..."

# Initialize counters
ERROR_COUNT=0
WARNING_COUNT=0

# Create necessary directory
mkdir -p /var/run/pullpiri

# Basic system checks
CPU_LOAD=$(cat /proc/loadavg | awk '{print $1}')
echo "CPU Load: $CPU_LOAD"

# Set status ready
echo "status=ready" > /var/run/pullpiri/node_status
echo "Default system check completed with $ERROR_COUNT errors and $WARNING_COUNT warnings."
exit 0
EOF
fi

chmod +x /usr/local/bin/node_ready_check.sh

# Create configuration file
echo "Creating configuration file..."
cat > ${CONFIG_DIR}/nodeagent.yaml << EOF
nodeagent:
  node_name: "${NODE_NAME}"
  node_type: "${NODE_TYPE}"
  node_role: "${NODE_ROLE}"
  master_ip: "${MASTER_IP}"
  node_ip: "${NODE_IP}"
  grpc_port: ${NODEAGENT_PORT}
  log_level: "info"
  metrics:
    collection_interval: 5
    batch_size: 50
  system:
    hostname: "$(hostname)"
    platform: "$(uname -s)"
    architecture: "$(uname -m)"
yaml_storage: "${YAML_STORAGE_DIR}"
EOF

# Check and add firewall rules
echo "Checking firewall configuration..."
if command -v firewall-cmd &> /dev/null && firewall-cmd --state &> /dev/null; then
    echo "firewalld is running. Allowing required ports."
    # Allow gRPC and other necessary ports
    if firewall-cmd --permanent --add-port=${NODEAGENT_PORT}/tcp; then
        firewall-cmd --reload
        echo "Firewall configuration has been updated."
    else
        echo "Warning: Failed to update firewall rules. You may need to add ports manually."
        WARNING_COUNT=$((WARNING_COUNT+1))
    fi
elif command -v ufw &> /dev/null && ufw status &> /dev/null; then
    echo "ufw is running. Allowing required ports."
    if ufw allow ${NODEAGENT_PORT}/tcp; then
        echo "Firewall configuration has been updated."
    else
        echo "Warning: Failed to update firewall rules. You may need to add ports manually."
        WARNING_COUNT=$((WARNING_COUNT+1))
    fi
else
    echo "No supported firewall manager found or not active."
fi

# Create systemd service file
echo "Creating systemd service file..."
cat > /etc/systemd/system/nodeagent.service << EOF
[Unit]
Description=Pullpiri NodeAgent Service
After=network-online.target
Wants=podman.socket

[Service]
Type=simple
ExecStartPre=/usr/local/bin/node_ready_check.sh ${NODE_TYPE}
ExecStart=${INSTALL_DIR}/${BINARY_NAME} --config ${CONFIG_DIR}/nodeagent.yaml
Restart=on-failure
RestartSec=10
Environment=RUST_LOG=info
Environment=MASTER_NODE_IP=${MASTER_IP}
Environment=NODE_IP=${NODE_IP}
Environment=GRPC_PORT=${NODEAGENT_PORT}

# Security hardening settings
ProtectSystem=full
ProtectHome=true
NoNewPrivileges=true

ReadWritePaths=/etc/pullpiri
ReadWritePaths=/etc/containers/systemd

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd and enable service
echo "Enabling NodeAgent service..."
systemctl daemon-reload
systemctl enable nodeagent.service || {
    echo "Error: Failed to enable NodeAgent service."
    ERROR_COUNT=$((ERROR_COUNT+1))
}

# Test master node connection before starting service
echo "Testing master node connection..."
if ping -c 3 -W 2 ${MASTER_IP} &> /dev/null; then
    echo "Master node is reachable: ${MASTER_IP}"
    
    # Check gRPC port
    if command -v nc &> /dev/null && nc -z -w 5 ${MASTER_IP} ${APISERVER_PORT} &> /dev/null; then
        echo "Master node gRPC port is accessible: ${MASTER_IP}:${APISERVER_PORT}"
    else
        echo "Warning: Unable to connect to master node gRPC port: ${MASTER_IP}:${APISERVER_PORT}"
        echo "Service will be registered but may wait until connection is available."
        WARNING_COUNT=$((WARNING_COUNT+1))
    fi
    
    # Check etcd port
   # if command -v nc &> /dev/null && nc -z -w 5 ${MASTER_IP} 2379 &> /dev/null; then
   #    echo "Master node etcd port is accessible: ${MASTER_IP}:2379"
   # else
   #     echo "Warning: Unable to connect to master node etcd port: ${MASTER_IP}:2379"
   #     echo "Service will be registered but may wait until connection is available."
   #     WARNING_COUNT=$((WARNING_COUNT+1))
   # fi
    
    # Start service
    echo "Starting NodeAgent service..."
    systemctl start nodeagent.service || {
        echo "Warning: Failed to start NodeAgent service."
        WARNING_COUNT=$((WARNING_COUNT+1))
    }
else
    echo "Warning: Unable to reach master node: ${MASTER_IP}"
    echo "Service will be registered but not started. Start manually when connection is available."
    WARNING_COUNT=$((WARNING_COUNT+1))
fi

# Check installation result
if systemctl is-enabled --quiet nodeagent.service; then
    echo "NodeAgent service has been registered with the system."
    
    if systemctl is-active --quiet nodeagent.service; then
        echo "NodeAgent service has been successfully started!"
    else
        echo "Warning: NodeAgent service is registered but not started."
        echo "Check logs: journalctl -u nodeagent.service"
        echo "Start manually after troubleshooting: systemctl start nodeagent.service"
        WARNING_COUNT=$((WARNING_COUNT+1))
    fi
else
    echo "Error: Failed to register NodeAgent service."
    ERROR_COUNT=$((ERROR_COUNT+1))
fi

echo "Installation Summary:"
echo "- Installation directory: ${INSTALL_DIR}"
echo "- Configuration file: ${CONFIG_DIR}/nodeagent.yaml"
echo "- Master node: ${MASTER_IP}:${APISERVER_PORT}"
echo "- Node IP: ${NODE_IP}"
echo "- Node type: ${NODE_TYPE}"
echo "- Status: $ERROR_COUNT errors, $WARNING_COUNT warnings"

if [ $ERROR_COUNT -gt 0 ]; then
    echo "Complete with errors: NodeAgent installation encountered problems that need to be addressed."
    exit 1
elif [ $WARNING_COUNT -gt 0 ]; then
    echo "Complete with warnings: NodeAgent installation finished with some non-critical issues."
    exit 0
else
    echo "Complete: NodeAgent installation process has finished successfully."
    exit 0
fi
