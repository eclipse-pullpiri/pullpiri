#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

# Basic GPU device mount test script
# Tests if GPU devices are properly mounted in the container

set -e

echo "=========================================="
echo "GPU Device Mount Test"
echo "=========================================="
echo ""

# Test 1: Check GPU device files
echo "Test 1: Checking GPU device files..."

FOUND_COUNT=0

# Check each device individually (POSIX compatible)
if [ -e "/dev/nvidia0" ]; then
    echo "  ✓ Found: /dev/nvidia0"
    FOUND_COUNT=$((FOUND_COUNT + 1))
else
    echo "  ✗ Missing: /dev/nvidia0"
fi

if [ -e "/dev/nvidiactl" ]; then
    echo "  ✓ Found: /dev/nvidiactl"
    FOUND_COUNT=$((FOUND_COUNT + 1))
else
    echo "  ✗ Missing: /dev/nvidiactl"
fi

if [ -e "/dev/nvidia-uvm" ]; then
    echo "  ✓ Found: /dev/nvidia-uvm"
    FOUND_COUNT=$((FOUND_COUNT + 1))
else
    echo "  ✗ Missing: /dev/nvidia-uvm"
fi

if [ -e "/dev/nvidia-uvm-tools" ]; then
    echo "  ✓ Found: /dev/nvidia-uvm-tools"
    FOUND_COUNT=$((FOUND_COUNT + 1))
else
    echo "  ✗ Missing: /dev/nvidia-uvm-tools"
fi

if [ -e "/dev/nvidia-modeset" ]; then
    echo "  ✓ Found: /dev/nvidia-modeset"
    FOUND_COUNT=$((FOUND_COUNT + 1))
else
    echo "  ✗ Missing: /dev/nvidia-modeset"
fi

echo ""
if [ $FOUND_COUNT -eq 0 ]; then
    echo "❌ FAILED: No GPU devices found!"
    exit 1
elif [ $FOUND_COUNT -lt 2 ]; then
    echo "⚠️  WARNING: Only $FOUND_COUNT device(s) found"
else
    echo "✓ PASSED: Found $FOUND_COUNT GPU device(s)"
fi

echo ""
echo "Test 2: Checking NVIDIA capability devices..."
if [ -d "/dev/nvidia-caps" ]; then
    echo "  ✓ Found: /dev/nvidia-caps/"
    ls -la /dev/nvidia-caps/
else
    echo "  ✗ Not found: /dev/nvidia-caps/ (optional)"
fi

echo ""
echo "Test 3: Checking environment variables..."
if [ -n "$NVIDIA_VISIBLE_DEVICES" ]; then
    echo "  ✓ NVIDIA_VISIBLE_DEVICES=$NVIDIA_VISIBLE_DEVICES"
else
    echo "  ✗ NVIDIA_VISIBLE_DEVICES not set"
fi

if [ -n "$NVIDIA_DRIVER_CAPABILITIES" ]; then
    echo "  ✓ NVIDIA_DRIVER_CAPABILITIES=$NVIDIA_DRIVER_CAPABILITIES"
else
    echo "  ✗ NVIDIA_DRIVER_CAPABILITIES not set"
fi

echo ""
echo "=========================================="
echo "Basic Device Mount Test: PASSED ✓"
echo "=========================================="
