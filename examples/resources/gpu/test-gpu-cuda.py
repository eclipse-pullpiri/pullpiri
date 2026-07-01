#!/usr/bin/env python3
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

"""
Simple GPU test script using CUDA
Tests if CUDA is available and can access GPU devices
"""

import sys

def test_cuda_availability():
    """Test if CUDA is available through PyTorch or other libraries"""
    print("=" * 50)
    print("GPU CUDA Functionality Test")
    print("=" * 50)
    print()
    
    # Test 1: Try PyTorch (most common)
    print("Test 1: Checking PyTorch CUDA support...")
    try:
        import torch
        print(f"  ✓ PyTorch version: {torch.__version__}")
        
        if torch.cuda.is_available():
            print(f"  ✓ CUDA available: True")
            print(f"  ✓ CUDA version: {torch.version.cuda}")
            print(f"  ✓ GPU device count: {torch.cuda.device_count()}")
            
            for i in range(torch.cuda.device_count()):
                print(f"\n  GPU {i}:")
                print(f"    Name: {torch.cuda.get_device_name(i)}")
                print(f"    Compute Capability: {torch.cuda.get_device_capability(i)}")
                
                # Get memory info
                total_memory = torch.cuda.get_device_properties(i).total_memory
                print(f"    Total Memory: {total_memory / (1024**3):.2f} GB")
            
            # Try simple computation
            print("\n  Test: Running simple tensor computation on GPU...")
            try:
                x = torch.rand(1000, 1000).cuda()
                y = torch.rand(1000, 1000).cuda()
                z = x @ y
                print(f"  ✓ Matrix multiplication successful: {z.shape}")
                print("  ✓ GPU computation WORKS!")
            except Exception as e:
                print(f"  ✗ GPU computation failed: {e}")
                return False
            
            return True
        else:
            print("  ✗ CUDA not available")
            return False
            
    except ImportError:
        print("  ⚠️  PyTorch not installed")
    except Exception as e:
        print(f"  ✗ PyTorch test failed: {e}")
    
    # Test 2: Try TensorFlow
    print("\nTest 2: Checking TensorFlow GPU support...")
    try:
        import tensorflow as tf
        print(f"  ✓ TensorFlow version: {tf.__version__}")
        
        gpus = tf.config.list_physical_devices('GPU')
        if gpus:
            print(f"  ✓ GPU devices found: {len(gpus)}")
            for gpu in gpus:
                print(f"    {gpu}")
            return True
        else:
            print("  ✗ No GPU devices found")
            
    except ImportError:
        print("  ⚠️  TensorFlow not installed")
    except Exception as e:
        print(f"  ✗ TensorFlow test failed: {e}")
    
    # Test 3: Try pycuda
    print("\nTest 3: Checking PyCUDA...")
    try:
        import pycuda.driver as cuda
        import pycuda.autoinit
        
        print(f"  ✓ PyCUDA available")
        print(f"  ✓ CUDA driver version: {cuda.get_driver_version()}")
        print(f"  ✓ Device count: {cuda.Device.count()}")
        
        for i in range(cuda.Device.count()):
            dev = cuda.Device(i)
            print(f"\n  GPU {i}:")
            print(f"    Name: {dev.name()}")
            print(f"    Compute Capability: {dev.compute_capability()}")
            print(f"    Total Memory: {dev.total_memory() / (1024**3):.2f} GB")
        
        return True
        
    except ImportError:
        print("  ⚠️  PyCUDA not installed")
    except Exception as e:
        print(f"  ✗ PyCUDA test failed: {e}")
    
    # Test 4: Try cupy
    print("\nTest 4: Checking CuPy...")
    try:
        import cupy as cp
        
        print(f"  ✓ CuPy version: {cp.__version__}")
        print(f"  ✓ CUDA version: {cp.cuda.runtime.runtimeGetVersion()}")
        
        # Try simple computation
        x = cp.array([1, 2, 3])
        y = cp.array([4, 5, 6])
        z = x + y
        print(f"  ✓ Simple computation successful: {z}")
        
        return True
        
    except ImportError:
        print("  ⚠️  CuPy not installed")
    except Exception as e:
        print(f"  ✗ CuPy test failed: {e}")
    
    print("\n" + "=" * 50)
    print("❌ FAILED: No CUDA library available or working")
    print("\nTo install CUDA libraries:")
    print("  PyTorch:    pip install torch")
    print("  TensorFlow: pip install tensorflow")
    print("  PyCUDA:     pip install pycuda")
    print("  CuPy:       pip install cupy-cuda11x")
    print("=" * 50)
    return False

if __name__ == "__main__":
    success = test_cuda_availability()
    
    print("\n" + "=" * 50)
    if success:
        print("✓ GPU Test PASSED: CUDA is working!")
    else:
        print("✗ GPU Test FAILED: CUDA not available")
    print("=" * 50)
    
    sys.exit(0 if success else 1)
