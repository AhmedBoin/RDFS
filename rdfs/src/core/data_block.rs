//! # RDFS DataBlock Module
//!
//! This module defines the `DataBlock` structure, which represents the actual
//! stored data in the RDFS file system. Each block contains metadata used for
//! integrity verification and time-based proofs, along with the actual data payload.
//!
//! ## Structure Overview
//! A `DataBlock` encapsulates:
//! - `block_number`: Unique nonce-like identifier (used as a spacetime proof dimension)
//! - `timestamp`: Creation or last-write time (used in auditing or replication)
//! - `data`: Raw payload content, excluding the reserved trailing metadata
//! - `signature`: 64-byte signature for attestation and zero-knowledge proof integrations
//!
//! ## Byte Layout
//! The encoded layout (total = `block_size` bytes):
//! ```text
//! [8 bytes: block_number]
//! [8 bytes: timestamp]
//! [8 bytes: data length]
//! [N bytes: data]
//! [padding up to block_size - 64]
//! [64 bytes: signature]
//! ```
//!
//! ## Design Goals
//! - Support for **proof-of-spacetime** via block_number and timestamp
//! - Data separation from metadata for integrity preservation
//! - Fixed-size encoding to simplify file system offset calculations
//!
//! ## Notes
//! - Signature must be externally generated and inserted using `add_signature`
//! - RaptorQ-related metadata (for erasure coding) is stored inside the `data` payload
//! - This block is reusable across shared and private file systems
//!
//! Copyrights © 2025 RDFS Contributors. All rights reserved.

use super::super::constants::{RESERVED_DB, SIG_SIZE, Signature};
use super::super::rdfs_errors::RDFSError;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct DataBlock {
    // block_size - 88 bytes
    pub block_number: u64, // Nonce for the block, used for proof of spacetime (block id) "first dimension".
    pub timestamp: u64,    // Timestamp for the block, used for proof of spacetime "second dimension".
    pub data: Vec<u8>,     // third dimension is integrated in RaptorQ first 4 bytes.
    pub signature: Signature,
}

impl DataBlock {
    pub fn new(block_number: u64, timestamp: u64, data: &[u8]) -> Self {
        Self {
            block_number,
            timestamp,
            data: data.to_vec(),
            signature: [0; SIG_SIZE],
        }
    }

    /// signing algorithm is not included in the file system.
    /// add your signature after removing last 64 bytes and
    /// exchange it with your signature
    pub fn add_signature(&mut self, signature: Signature) {
        self.signature = signature;
    }

    pub fn to_bytes(&self, block_size: usize) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(block_size);

        encoded.extend_from_slice(&self.block_number.to_le_bytes());
        encoded.extend_from_slice(&self.timestamp.to_le_bytes());
        encoded.extend_from_slice(&(self.data.len() as u64).to_le_bytes());
        encoded.extend_from_slice(&self.data);
        encoded.resize(block_size - SIG_SIZE, 0);
        encoded.extend_from_slice(&self.signature);

        encoded
    }

    pub fn from_bytes(data: &[u8], block_size: usize) -> Result<Self> {
        if data.len() != block_size {
            return Err(RDFSError::InvalidDataBlockLength.into());
        }

        let block_number = u64::from_le_bytes(data[..8].try_into().unwrap());
        let timestamp = u64::from_le_bytes(data[8..16].try_into().unwrap());

        let length = u64::from_le_bytes(data[16..24].try_into().unwrap()) as usize;
        if length > block_size - RESERVED_DB {
            return Err(RDFSError::InvalidEncodedDataBlockLength.into());
        }

        let mut content = Vec::with_capacity(length);
        content.extend_from_slice(&data[24..data.len() - SIG_SIZE]);
        let signature: Signature = data[block_size - SIG_SIZE..].try_into().unwrap();

        Ok(Self {
            block_number,
            timestamp,
            data: content,
            signature,
        })
    }
}
