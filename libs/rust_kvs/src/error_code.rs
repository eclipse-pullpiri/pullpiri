// Copyright (c) 2025 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// <https://www.apache.org/licenses/LICENSE-2.0>
//
// SPDX-License-Identifier: Apache-2.0

extern crate alloc;

use alloc::string::FromUtf8Error;
use core::array::TryFromSliceError;

/// Runtime Error Codes
#[derive(Debug, PartialEq)]
pub enum ErrorCode {
    /// Error that was not yet mapped
    UnmappedError,

    /// File not found
    FileNotFound,

    /// KVS file read error
    KvsFileReadError,

    /// KVS hash file read error
    KvsHashFileReadError,

    /// JSON parser error
    JsonParserError,

    /// JSON generator error
    JsonGeneratorError,

    /// Physical storage failure
    PhysicalStorageFailure,

    /// Integrity corrupted
    IntegrityCorrupted,

    /// Validation failed
    ValidationFailed,

    /// Encryption failed
    EncryptionFailed,

    /// Resource is busy
    ResourceBusy,

    /// Out of storage space
    OutOfStorageSpace,

    /// Quota exceeded
    QuotaExceeded,

    /// Authentication failed
    AuthenticationFailed,

    /// Key not found
    KeyNotFound,

    // Key has no default value
    KeyDefaultNotFound,

    /// Serialization failed
    SerializationFailed,

    /// Invalid snapshot ID
    InvalidSnapshotId,

    /// Invalid instance ID
    InvalidInstanceId,

    /// Conversion failed
    ConversionFailed,

    /// Mutex failed
    MutexLockFailed,

    /// Instance parameters mismatch
    InstanceParametersMismatch,
}

impl From<std::io::Error> for ErrorCode {
    fn from(cause: std::io::Error) -> Self {
        let kind = cause.kind();
        match kind {
            std::io::ErrorKind::NotFound => ErrorCode::FileNotFound,
            _ => {
                eprintln!("error: unmapped error: {kind}");
                ErrorCode::UnmappedError
            }
        }
    }
}

impl From<FromUtf8Error> for ErrorCode {
    fn from(cause: FromUtf8Error) -> Self {
        eprintln!("error: UTF-8 conversion failed: {cause:#?}");
        ErrorCode::ConversionFailed
    }
}

impl From<TryFromSliceError> for ErrorCode {
    fn from(cause: TryFromSliceError) -> Self {
        eprintln!("error: try_into from slice failed: {cause:#?}");
        ErrorCode::ConversionFailed
    }
}

impl From<Vec<u8>> for ErrorCode {
    fn from(cause: Vec<u8>) -> Self {
        eprintln!("error: try_into from u8 vector failed: {cause:#?}");
        ErrorCode::ConversionFailed
    }
}

#[cfg(test)]
mod error_code_tests {
    use crate::error_code::ErrorCode;
    use std::io::{Error, ErrorKind};

    #[test]
    fn test_from_io_error_to_file_not_found() {
        let error = Error::new(ErrorKind::NotFound, "File not found");
        assert_eq!(ErrorCode::from(error), ErrorCode::FileNotFound);
    }

    #[test]
    fn test_from_io_error_to_unmapped_error() {
        let error = std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid input provided");
        assert_eq!(ErrorCode::from(error), ErrorCode::UnmappedError);
    }

    #[test]
    fn test_from_utf8_error_to_conversion_failed() {
        // test from: https://doc.rust-lang.org/std/string/struct.FromUtf8Error.html
        let bytes = vec![0, 159];
        let error = String::from_utf8(bytes).unwrap_err();
        assert_eq!(ErrorCode::from(error), ErrorCode::ConversionFailed);
    }

    #[test]
    fn test_from_try_from_slice_error_to_conversion_failed() {
        let bytes = [0x12, 0x34, 0x56, 0x78, 0xab];
        let bytes_ptr: &[u8] = &bytes;
        let error = TryInto::<[u8; 8]>::try_into(bytes_ptr).unwrap_err();
        assert_eq!(ErrorCode::from(error), ErrorCode::ConversionFailed);
    }

    #[test]
    fn test_from_vec8_to_conversion_failed() {
        let bytes: Vec<u8> = vec![];
        assert_eq!(ErrorCode::from(bytes), ErrorCode::ConversionFailed);
    }
}
