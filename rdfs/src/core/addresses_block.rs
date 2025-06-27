//! # RDFS AddressesBlock Module
//!
//! This module defines the `AddressesBlock` structure used to store public keys
//! representing participating nodes in the RDFS virtual drive.
//!
//! Each `AddressesBlock` includes:
//! - A dynamic list of `Address` entries (typically 32-byte public keys)
//! - A final 64-byte `Signature` used for authentication or integrity verification
//!
//! The `AddressesBlock` is stored immediately after the `SuperBlock` and is
//! essential for drive-level identity management, quorum verification, or
//! consensus-based validation among nodes.
//!
//! ## Features
//! - Compact, length-prefixed encoding of node public keys
//! - Fixed-size, signature-terminated layout
//! - Strict size checks to ensure determinism and forward compatibility
//! - Manual signature attachment for ZK/STARK-friendly workflows
//!
//! ## Encoding Layout
//! ```text
//! [8 bytes: length][32 bytes * N: addresses][64 bytes: signature]
//! ```
//!
//! ## Design Goals
//! - Keep representation flat for efficient I/O
//! - Separate cryptographic responsibilities from data structure
//! - Maintain byte compatibility across node implementations
//!
//! Copyrights © 2025 RDFS Contributors. All rights reserved.

use super::super::constants::{Address, PK_SIZE, RESERVED_AB, SIG_SIZE, Signature};
use super::super::rdfs_errors::RDFSError;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AddressesBlock {
    // 72 + 32 * nodes bytes
    pub addresses: Vec<Address>,
    pub signature: Signature, // Signature for the block
}

impl AddressesBlock {
    /// Create a new AddressesBlock that can hold `size` bytes worth of addresses.
    /// The `size` should be divisible by 32.
    pub fn new(addresses: Vec<Address>, signature: Signature) -> Self {
        Self { addresses, signature }
    }

    /// signing algorithm is not included in the file system.
    /// add your signature after removing last 64 bytes and
    /// exchange it with your signature
    pub fn add_signature(&mut self, signature: Signature) {
        self.signature = signature;
    }

    /// Serialize to a flat byte array
    pub fn to_bytes(&self) -> Vec<u8> {
        let nodes_address_size = RESERVED_AB + PK_SIZE * self.addresses.len();
        let mut encoded = Vec::with_capacity(nodes_address_size);

        encoded.extend_from_slice(&(self.addresses.len() as u64).to_le_bytes());
        for address in self.addresses.iter() {
            encoded.extend_from_slice(address);
        }
        encoded.extend_from_slice(&self.signature);

        encoded
    }

    pub fn from_bytes(data: &[u8], nodes_address_size: usize) -> Result<Self> {
        if data.len() != nodes_address_size {
            return Err(RDFSError::InvalidAddressBlockLength.into());
        }

        let length = u64::from_le_bytes(data[..8].try_into().unwrap()) as usize;

        if RESERVED_AB + PK_SIZE * length != nodes_address_size {
            return Err(RDFSError::InvalidEncodedAddressBlockLength.into());
        }

        let mut addresses = Vec::with_capacity(length);
        for i in 0..length {
            let start = 8 + i * PK_SIZE;
            let end = start + PK_SIZE;
            let mut address = [0u8; PK_SIZE];
            address.copy_from_slice(&data[start..end]);
            addresses.push(address);
        }

        let signature: Signature = data[nodes_address_size - SIG_SIZE..].try_into().unwrap();

        Ok(Self { addresses, signature })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn addresses_block_serialization_test() {
        let addresses = vec![[1u8; PK_SIZE], [2u8; PK_SIZE], [3u8; PK_SIZE], [4u8; PK_SIZE]];
        let signature = [5u8; SIG_SIZE];
        let block = AddressesBlock::new(addresses, signature);

        // Serialize the block
        let serialized = block.to_bytes();
        println!("Serialized AddressesBlock: {:?}", serialized.len());

        // Deserialize back to a block
        let deserialized = AddressesBlock::from_bytes(&serialized, serialized.len()).unwrap();

        // Check if the original and deserialized blocks are equal
        assert_eq!(block.addresses, deserialized.addresses);
        assert_eq!(block.signature, deserialized.signature);
    }
}
