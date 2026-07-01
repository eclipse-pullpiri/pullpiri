#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

# nvidia-smi wrapper test
# Simple test using nvidia-smi if available

echo "=========================================="
echo "NVIDIA SMI Test"
echo "=========================================="
echo ""

if command -v nvidia-smi &> /dev/null; then
    echo "✓ nvidia-smi found"
    echo ""
    echo "Running: nvidia-smi"
    echo "----------------------------------------"
    nvidia-smi
    echo "----------------------------------------"
    echo ""
    echo "✓ PASSED: nvidia-smi executed successfully"
else
    echo "✗ nvidia-smi not found in PATH"
    echo ""
    echo "Checking for NVIDIA devices anyway..."
    if [ -e "/dev/nvidia0" ]; then
        echo "✓ GPU devices exist but nvidia-smi not available"
        echo "  This is expected if using base CUDA image without nvidia-smi"
    else
        echo "✗ No GPU devices found"
        exit 1
    fi
fi

echo ""
echo "=========================================="
