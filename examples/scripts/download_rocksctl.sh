# Check root privileges
if [ "$(id -u)" -ne 0 ]; then
    echo "Error: This script must be run as root."
    exit 1
fi

ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then
    BINARY_SUFFIX="linux-amd64"
else
    BINARY_SUFFIX="linux-arm64"
fi

wget https://github.com/MCO-PICCOLO/rocksctl/releases/latest/download/rocksctl-${BINARY_SUFFIX}

sudo mv rocksctl-${BINARY_SUFFIX} /usr/bin/rocksctl
sudo chmod 755 /usr/bin/rocksctl
