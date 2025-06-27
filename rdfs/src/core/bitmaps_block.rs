//! # RDFS BitmapsBlock Module
//!
//! This module defines the `BitmapsBlock` structure responsible for tracking block
//! allocation status in a shared RDFS file system.
//!
//! It acts as a space-efficient bitmap (bit field) where each bit corresponds
//! to the usage status of a data block. This mechanism enables fast and compact
//! bookkeeping over large storage volumes with minimal overhead.
//!
//! ## Layout
//! The layout of a serialized `BitmapsBlock` is:
//! ```text
//! [8 bytes: total_blocks]
//! [8 bytes: free_blocks]
//! [8 bytes: last_modify_timestamp]
//! [8 bytes: bit_field length]
//! [N bytes: bit_field (N = total_blocks / 8)]
//! [64 bytes: signature]
//! ```
//!
//! ## Features
//! - Efficient per-block allocation tracking
//! - Self-contained timestamp for last modification
//! - Manual signature field for future verification (e.g., proof-of-spacetime)
//!
//! ## Use Cases
//! - File system consistency checking
//! - Fault tolerance with redundancy tracking
//! - ZK/STARK-compatible designs with signature append-only logic
//!
//! ## Design Considerations
//! - Shared RDFS only; not used in Private RDFS
//! - Signature is not auto-generated—intended for external prover logic
//! - Modifying bit flags updates the last-modified timestamp automatically
//!
//! Copyrights © 2025 RDFS Contributors. All rights reserved.

use super::super::constants::{RESERVED_BB, SIG_SIZE, Signature};
use super::super::utils::current_time_as_u64;
use super::super::rdfs_errors::RDFSError;
use anyhow::Result;

/// A block representing a bitmap for tracking allocation of blocks/nodes.
/// Internally stores a `Vec<u8>` of size `block_size`.
#[derive(Debug, Clone)]
pub struct BitmapsBlock {
    // 96 + total_blocks / 8 bytes
    pub total_blocks: u64, // Total number of blocks in the filesystem
    pub free_blocks: u64,  // Number of free blocks available
    pub last_modify: u64,  // Timestamp of the last modification
    pub bit_field: Vec<u8>,
    pub signature: Signature,
}

impl BitmapsBlock {
    /// Creates a new BitmapsBlock with all bits set to 0.
    pub fn new(total_blocks: u64, timestamp: u64) -> Self {
        Self {
            total_blocks,
            free_blocks: total_blocks,
            last_modify: timestamp,
            bit_field: vec![0; (total_blocks / 8) as usize],
            signature: [0; SIG_SIZE],
        }
    }

    /// signing algorithm is not included in the file system.
    /// add your signature after removing last 64 bytes and
    /// exchange it with your signature
    pub fn add_signature(&mut self, signature: Signature) {
        self.signature = signature;
    }

    /// Returns `true` if the bit at `bit_index` is set.
    pub fn get_bit(&self, bit_index: usize) -> bool {
        let byte = bit_index / 8;
        let bit = bit_index % 8;
        if byte >= self.bit_field.len() {
            return false;
        }
        (self.bit_field[byte] & (1 << bit)) != 0
    }

    /// Sets the bit at `bit_index` to 1, and increments free_blocks only if it was 0.
    pub fn set_bit(&mut self, bit_index: usize) {
        let byte = bit_index / 8;
        let bit = bit_index % 8;
        if byte < self.bit_field.len() {
            let mask = 1 << bit;
            if self.bit_field[byte] & mask == 0 {
                self.bit_field[byte] |= mask;
                self.free_blocks -= 1;
                if let Ok(time) = current_time_as_u64() {
                    self.last_modify = time
                }
            }
        }
    }

    /// Clears the bit at `bit_index` to 0, and decrements free_blocks only if it was 1.
    pub fn clear_bit(&mut self, bit_index: usize) {
        let byte = bit_index / 8;
        let bit = bit_index % 8;
        if byte < self.bit_field.len() {
            let mask = 1 << bit;
            if self.bit_field[byte] & mask != 0 {
                self.bit_field[byte] &= !mask;
                self.free_blocks += 1;
                if let Ok(time) = current_time_as_u64() {
                    self.last_modify = time
                }
            }
        }
    }

    /// Serialize the entire bitmap to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let bitmaps_size = RESERVED_BB + (self.total_blocks / 8) as usize;
        let mut encoded = Vec::with_capacity(bitmaps_size);

        encoded.extend_from_slice(&self.total_blocks.to_le_bytes());
        encoded.extend_from_slice(&self.free_blocks.to_le_bytes());
        encoded.extend_from_slice(&self.last_modify.to_le_bytes());
        encoded.extend_from_slice(&(self.bit_field.len() as u64).to_le_bytes());
        encoded.extend_from_slice(&self.bit_field);
        encoded.extend_from_slice(&self.signature);

        encoded
    }

    /// Deserialize a BitmapsBlock from raw bytes.
    pub fn from_bytes(data: &[u8], bitmaps_size: usize) -> Result<Self> {
        if data.len() != bitmaps_size {
            return Err(RDFSError::InvalidBitmapsBlockLength.into());
        }

        let total_blocks = u64::from_le_bytes(data[..8].try_into().unwrap());
        let free_blocks = u64::from_le_bytes(data[8..16].try_into().unwrap());
        let last_modify = u64::from_le_bytes(data[16..24].try_into().unwrap());
        let length = u64::from_le_bytes(data[24..32].try_into().unwrap()) as usize;

        if 96 + length != bitmaps_size {
            return Err(RDFSError::InvalidEncodedBitmapsBlockLength.into());
        }

        let mut bit_field = Vec::with_capacity(length);
        bit_field.extend_from_slice(&data[32..data.len() - SIG_SIZE]);
        let signature: Signature = data[bitmaps_size - SIG_SIZE..].try_into().unwrap();

        Ok(Self {
            total_blocks,
            free_blocks,
            last_modify,
            bit_field,
            signature,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bitmaps_block_serialization_test() {
        let timestamp = 1633036800; // Example timestamp
        let total_blocks = 1024;
        let mut block = BitmapsBlock::new(total_blocks, timestamp);

        // Set some bits
        block.set_bit(0);
        block.set_bit(10);
        block.set_bit(20);

        // Serialize the block
        let serialized = block.to_bytes();
        println!("Serialized BitmapsBlock: {:?}", serialized.len());

        // Deserialize back to a block
        let deserialized = BitmapsBlock::from_bytes(&serialized, RESERVED_BB + (total_blocks / 8) as usize).unwrap();

        // Check if the original and deserialized blocks are equal
        assert_eq!(block.total_blocks, deserialized.total_blocks);
        assert_eq!(block.free_blocks, deserialized.free_blocks);
        assert_eq!(block.last_modify, deserialized.last_modify);
        assert_eq!(block.bit_field, deserialized.bit_field);
    }
}
