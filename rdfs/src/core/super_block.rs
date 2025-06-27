//! # RDFS SuperBlock Module
//!
//! This module defines the core `SuperBlock` structure, which serves as the root metadata
//! for every virtual RDFS drive. It contains critical layout and configuration data for the
//! file system, including size, block allocation, node layout, pointer offsets, and redundancy strategy.
//!
//! The `SuperBlock` enables consistent parsing and validation of a distributed drive's structure
//! across different nodes and chains, making it fundamental to both shared and private variants
//! of RDFS storage.
//!
//! ## Features
//! - Shared and Private drive layout generation
//! - Redundancy-aware storage calculations
//! - Pointer map for in-place block access
//! - Signature field for proof-of-spacetime integrity
//! - Forward/backward compatible byte serialization
//!
//! ## Design Philosophy
//! The structure is designed to:
//! - Be compact and efficiently serializable to fixed-length byte arrays
//! - Allow deterministic mounting and verification from raw byte slices
//! - Abstract over future cryptographic or structural changes
//!
//! ## Key Fields
//! - `magic`: Distinguishes between Shared and Private drives
//! - `inode_pointer`: Last block reserved for the root inode directory
//! - `signature`: Allows the entire super block to be signed/verified externally
//!
//! Copyrights © 2025 RDFS Contributors. All rights reserved.

use super::super::constants::{
    Address, CONTENT_SIZE, FS_MAGIC_PRIVATE, FS_MAGIC_SHARED, PK_SIZE, RESERVED_AB, RESERVED_BB, RESERVED_CDB, RESERVED_IB, RESERVED_LIB, SB_SIZE,
    Signature,
};
use anyhow::{Result, anyhow};
use core::f64::math::{ceil, floor};
use super::super::rdfs_errors::RDFSError;

/// Represents the SuperBlock — the root metadata structure of the file system.
/// Stores info about storage, nodes, block layout, some pointer and signature.
#[derive(Debug, Clone)]
pub struct SuperBlock {
    // 256 bytes
    pub magic: FileSystemType, // Magic word identifies the filesystem b"RDFS-***"
    pub owner: Address,        // Owner of the filesystem, usually the creator's public key
    pub program_id: Address,   // ID of the program that created the filesystem
    pub storage: u64,          // Total storage size in bytes
    pub redundancy: u64,       // Redundancy in percentage % without divided by 100 e.g., 300% for 3x redundancy
    pub nodes: u64,            // Total number of nodes holding the filesystem data
    pub block_size: u64,       // Size of each block in bytes
    pub total_blocks: u64,     // Total number of blocks in the filesystem

    // -- additional fields for quick data access --
    pub client_block_size: u64,     // size before encoding, used for client-side operations
    pub node_storage: u64,          // size of file located for virtual file system
    pub nodes_address_pointer: u64, // Pointer to the nodes address list
    pub bitmaps_pointer: u64,       // Pointer to the bitmaps
    pub data_pointer: u64,          // pointer to first data block
    pub inode_pointer: u64,         // Pointer to the inode table "root directory" (last block in the file system)

    pub nodes_address_size: u64,          // size in bytes starting from address pointer
    pub bitmaps_size: u64,                // size in bytes starting from bitmaps pointer
    pub max_content_pointers: u64,        // Maximum number of pointers inside inode table points to other blocks
    pub max_linked_content_pointers: u64, // Maximum number of pointers inside linked inode table points to other blocks

    pub signature: Signature, // Signature for the block, used for verification and proof of spacetime
}

impl SuperBlock {
    /// used for the first time when creating new virtual drive
    pub fn new(magic: FileSystemType, owner: Address, program_id: Address, storage: u64, redundancy: u64, nodes: u64, block_size: u64) -> Self {
        match magic {
            FileSystemType::Shared => Self::new_shared(magic, owner, program_id, storage, redundancy, nodes, block_size),
            FileSystemType::Private => Self::new_private(magic, owner, program_id, storage, redundancy, nodes, block_size),
        }
    }

    pub fn new_shared(
        magic: FileSystemType,
        owner: Address,
        program_id: Address,
        storage: u64,
        redundancy: u64,
        nodes: u64,
        block_size: u64,
    ) -> Self {
        // block_size - (signature + block_number + timestamp + data length + packet number "RaptorQ first 4 bytes")
        let block_size_for_data = block_size - (RESERVED_CDB as u64);
        let redundancy_ratio = redundancy as f64 / 100.0;

        let client_block_size = floor((block_size_for_data * nodes) as f64 / redundancy_ratio) as u64;

        // -----------------------------------------------------------------------------------------
        // ----------------------- calculating total blocks and bitmaps size -----------------------
        // -----------------------------------------------------------------------------------------
        // after finding initial value of node storage
        // remain storage = node storage - super block - address block + bitmaps metadata (96 bytes)
        // remain storage = (total blocks / 8) + (total block * block size)
        // remain storage = total blocks * (block size + 1/8)
        // total blocks = remain storage / (block size + 1/8)
        // ------------- corrected values to make each byte in bitmap point to 8 blocks -------------
        // total blocks = ceil(total blocks / 8) * 8
        // node storage = super block + address block + bitmaps metadata + (total blocks / 8) + (total blocks * block size)
        // ------------------------------------------------------------------------------------------

        let node_storage = storage as f64 * redundancy_ratio / nodes as f64;
        let remain_storage = node_storage - ((SB_SIZE as f64) + (RESERVED_AB as f64) + (PK_SIZE as f64) * nodes as f64 + (RESERVED_BB as f64));
        let total_blocks = remain_storage / (block_size as f64 + 0.125);
        // corrected values
        let total_blocks = ceil(total_blocks / 8.0) as u64 * 8;
        let node_storage = (SB_SIZE as u64)
            + (RESERVED_AB as u64)
            + (PK_SIZE as u64) * nodes
            + (RESERVED_BB as u64)
            + (total_blocks / 8)
            + total_blocks * block_size;

        let nodes_address_size = (RESERVED_AB as u64) + (PK_SIZE as u64) * nodes;
        let bitmaps_size = (RESERVED_BB as u64) + total_blocks / 8;

        let nodes_address_pointer = SB_SIZE as u64;
        let bitmaps_pointer = nodes_address_pointer + nodes_address_size;
        let data_pointer = bitmaps_pointer + bitmaps_size;
        let inode_pointer = data_pointer + block_size * (total_blocks - 1);

        let max_content_pointers = floor((block_size as f64 - (RESERVED_IB as f64)) / (CONTENT_SIZE as f64)) as u64;
        let max_linked_content_pointers = floor((block_size as f64 - (RESERVED_LIB as f64)) / (CONTENT_SIZE as f64)) as u64;

        Self {
            magic,
            owner,
            program_id,
            storage,
            redundancy,
            nodes,
            block_size,
            total_blocks,

            client_block_size,
            node_storage,
            nodes_address_pointer,
            bitmaps_pointer,
            data_pointer,
            inode_pointer,

            nodes_address_size,
            bitmaps_size,
            max_content_pointers,
            max_linked_content_pointers,

            signature: [0; 64],
        }
    }

    pub fn new_private(
        magic: FileSystemType,
        owner: Address,
        program_id: Address,
        storage: u64,
        redundancy: u64,
        nodes: u64,
        block_size: u64,
    ) -> Self {
        let redundancy_ratio = redundancy as f64 / 100.0;
        let node_storage = storage as f64 * redundancy_ratio / nodes as f64;
        let remain_storage = node_storage - ((SB_SIZE as f64) + (RESERVED_AB as f64) + (PK_SIZE as f64) * nodes as f64);
        let total_blocks = ceil(remain_storage / block_size as f64) as u64;
        // corrected values
        let node_storage = (SB_SIZE as u64) + (RESERVED_AB as u64) + (PK_SIZE as u64) * nodes + total_blocks * block_size;

        let nodes_address_size = (RESERVED_AB as u64) + (PK_SIZE as u64) * nodes;
        let nodes_address_pointer = SB_SIZE as u64;
        let data_pointer = nodes_address_pointer + nodes_address_size;

        Self {
            magic,
            owner,
            program_id,
            storage,
            redundancy,
            nodes,
            block_size,
            total_blocks,

            node_storage,
            nodes_address_pointer,
            data_pointer,

            nodes_address_size,
            client_block_size: 0,
            bitmaps_pointer: 0,
            inode_pointer: 0,
            bitmaps_size: 0,
            max_content_pointers: 0,
            max_linked_content_pointers: 0,

            signature: [0; 64],
        }
    }

    /// signing algorithm is not included in the file system.
    /// add your signature after removing last 64 bytes and
    /// exchange it with your signature
    pub fn add_signature(&mut self, signature: Signature) {
        self.signature = signature;
    }

    /// Serialize to prepare for storing or transmission.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(SB_SIZE);

        encoded.extend_from_slice(&self.magic.to_bytes());
        encoded.extend_from_slice(&self.owner);
        encoded.extend_from_slice(&self.program_id);
        encoded.extend_from_slice(&self.storage.to_le_bytes());
        encoded.extend_from_slice(&self.redundancy.to_le_bytes());
        encoded.extend_from_slice(&self.nodes.to_le_bytes());
        encoded.extend_from_slice(&self.block_size.to_le_bytes());
        encoded.extend_from_slice(&self.total_blocks.to_le_bytes());
        encoded.extend_from_slice(&self.client_block_size.to_le_bytes());
        encoded.extend_from_slice(&self.node_storage.to_le_bytes());
        encoded.extend_from_slice(&self.nodes_address_pointer.to_le_bytes());
        encoded.extend_from_slice(&self.bitmaps_pointer.to_le_bytes());
        encoded.extend_from_slice(&self.data_pointer.to_le_bytes());
        encoded.extend_from_slice(&self.inode_pointer.to_le_bytes());
        encoded.extend_from_slice(&self.nodes_address_size.to_le_bytes());
        encoded.extend_from_slice(&self.bitmaps_size.to_le_bytes());
        encoded.extend_from_slice(&self.max_content_pointers.to_le_bytes());
        encoded.extend_from_slice(&self.max_linked_content_pointers.to_le_bytes());
        encoded.extend_from_slice(&self.signature);

        encoded
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() != SB_SIZE {
            return Err(RDFSError::InvalidSuperBlockLength.into());
        }

        let magic = FileSystemType::from_bytes(&data[..8])?;
        let owner = data[8..40].try_into().unwrap();
        let program_id = data[40..72].try_into().unwrap();
        let storage = u64::from_le_bytes(data[72..80].try_into().unwrap());
        let redundancy = u64::from_le_bytes(data[80..88].try_into().unwrap());
        let nodes = u64::from_le_bytes(data[88..96].try_into().unwrap());
        let block_size = u64::from_le_bytes(data[96..104].try_into().unwrap());
        let total_blocks = u64::from_le_bytes(data[104..112].try_into().unwrap());
        let client_block_size = u64::from_le_bytes(data[112..120].try_into().unwrap());
        let node_storage = u64::from_le_bytes(data[120..128].try_into().unwrap());
        let nodes_address_pointer = u64::from_le_bytes(data[128..136].try_into().unwrap());
        let bitmaps_pointer = u64::from_le_bytes(data[136..144].try_into().unwrap());
        let data_pointer = u64::from_le_bytes(data[144..152].try_into().unwrap());
        let inode_pointer = u64::from_le_bytes(data[152..160].try_into().unwrap());
        let nodes_address_size = u64::from_le_bytes(data[160..168].try_into().unwrap());
        let bitmaps_size = u64::from_le_bytes(data[168..176].try_into().unwrap());
        let max_content_pointers = u64::from_le_bytes(data[176..184].try_into().unwrap());
        let max_linked_content_pointers = u64::from_le_bytes(data[184..192].try_into().unwrap());
        let signature = data[192..].try_into().unwrap();

        Ok(Self {
            magic,
            owner,
            program_id,
            storage,
            redundancy,
            nodes,
            block_size,
            total_blocks,
            client_block_size,
            node_storage,
            nodes_address_pointer,
            bitmaps_pointer,
            data_pointer,
            inode_pointer,
            nodes_address_size,
            bitmaps_size,
            max_content_pointers,
            max_linked_content_pointers,
            signature,
        })
    }
}

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSystemType {
    Shared = FS_MAGIC_SHARED,
    Private = FS_MAGIC_PRIVATE,
}

impl FileSystemType {
    pub fn to_bytes(&self) -> [u8; 8] {
        (*self as u64).to_le_bytes()
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() != 8 {
            return Err(anyhow!("Invalid length: expected 8 bytes"));
        }

        let mut arr = [0u8; 8];
        arr.copy_from_slice(data);
        let value = u64::from_le_bytes(arr);

        match value {
            FS_MAGIC_SHARED => Ok(FileSystemType::Shared),
            FS_MAGIC_PRIVATE => Ok(FileSystemType::Private),
            _ => Err(RDFSError::InvalidMagicWord.into()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::super::utils::bytes_to_hex;
    use super::*;

    #[test]
    fn new_super_block_test() {
        let owner = [255; 32];
        let program_id = [1; 32];
        let storage = 34359738368;
        let redundancy = 300;
        let nodes = 50;
        let block_size = 4096; // 4KB block size

        // instead of returning result, this fields will be checked on chain.
        assert!(
            storage >= nodes * 1048576,
            "minimum storage with respect to number of nodes should be >= nodes * 1MB"
        );
        assert!(redundancy >= 100, "Redundancy should be >= 100.");
        assert!(nodes >= 1, "minimum nodes to operate is 1.");
        assert!(
            block_size >= 2048,
            "minimum block size is 2KB but it will be not efficient ~90% of storage"
        );

        let block = super::SuperBlock::new(FileSystemType::Shared, owner, program_id, storage, redundancy, nodes, block_size);
        println!("magic: {:?}", block.magic);
        println!("program_id: 0x{}", bytes_to_hex(&block.program_id));
        println!("storage: {:?}", block.storage);
        println!("redundancy: {:?}", block.redundancy);
        println!("nodes: {:?}", block.nodes);
        println!("block_size: {:?}", block.block_size);
        println!("total_blocks: {:?}", block.total_blocks);
        println!("client_block_size: {:?}", block.client_block_size);
        println!("node_storage: {:?}", block.node_storage);
        println!("nodes_address_pointer: {:?}", block.nodes_address_pointer);
        println!("bitmaps_pointer: {:?}", block.bitmaps_pointer);
        println!("data_pointer: {:?}", block.data_pointer);
        println!("inode_pointer: {:?}", block.inode_pointer);
        println!("----------------------------");
        println!("nodes_address size: {:?}", block.nodes_address_size);
        println!("bitmaps size: {:?}", block.bitmaps_size);
        println!("max content pointers: {:?}", block.max_content_pointers);
        println!("max linked content pointers: {:?}", block.max_linked_content_pointers);
        println!("----------------------------");
        println!(
            "System Storage Efficiency: {:.2?}%",
            ((block.block_size - 88) * block.total_blocks * 100) as f64 / block.node_storage as f64
        );

        match block.magic {
            FileSystemType::Shared => {
                assert_eq!(
                    block.node_storage,
                    256 + block.nodes_address_size + block.bitmaps_size + block.total_blocks * block.block_size,
                    "node storage should be equal to super block + address block + bitmaps metadata + (total blocks / 8) + (total blocks * block size)"
                );
            }
            FileSystemType::Private => {
                assert_eq!(
                    block.node_storage,
                    256 + block.nodes_address_size + block.total_blocks * block.block_size,
                    "node storage should be equal to super block + address block + (total blocks * block size)"
                );
            }
        }
    }

    #[test]
    fn serialize_test() {
        let owner = [255; 32];
        let program_id = [1; 32];
        let storage = 34359738368;
        let redundancy = 300;
        let nodes = 50;
        let block_size = 4096;

        // instead of returning result, this fields will be checked on chain.
        assert!(
            storage >= nodes * 1048576,
            "minimum storage with respect to number of nodes should be >= nodes * 1MB"
        );
        assert!(redundancy >= 100, "Redundancy should be >= 100.");
        assert!(nodes >= 1, "minimum nodes to operate is 1.");
        assert!(
            block_size >= 2048,
            "minimum block size is 2KB but it will be not efficient ~90% of storage"
        );

        let block = SuperBlock::new(FileSystemType::Private, owner, program_id, storage, redundancy, nodes, block_size);

        let ser = block.to_bytes();
        println!("length: {:?}", ser.len());

        let block2 = SuperBlock::from_bytes(&ser).unwrap();
        assert_eq!(block.magic, block2.magic, "Magic number should match");
        assert_eq!(block.owner, block2.owner, "Owner should match");
        assert_eq!(block.program_id, block2.program_id, "Program ID should match");
        assert_eq!(block.storage, block2.storage, "Storage size should match");
        assert_eq!(block.redundancy, block2.redundancy, "Redundancy should match");
        assert_eq!(block.nodes, block2.nodes, "Number of nodes should match");
        assert_eq!(block.block_size, block2.block_size, "Block size should match");
        assert_eq!(block.total_blocks, block2.total_blocks, "Total blocks should match");
        assert_eq!(block.client_block_size, block2.client_block_size, "Client block size should match");
        assert_eq!(block.node_storage, block2.node_storage, "Node storage should match");
        assert_eq!(
            block.nodes_address_pointer, block2.nodes_address_pointer,
            "Nodes address pointer should match"
        );
        assert_eq!(block.bitmaps_pointer, block2.bitmaps_pointer, "Bitmaps pointer should match");
        assert_eq!(block.data_pointer, block2.data_pointer, "Data pointer should match");
        assert_eq!(block.inode_pointer, block2.inode_pointer, "Inode pointer should match");
        assert_eq!(block.nodes_address_size, block2.nodes_address_size, "Nodes address size should match");
        assert_eq!(block.bitmaps_size, block2.bitmaps_size, "Bitmaps size should match");
        assert_eq!(
            block.max_content_pointers, block2.max_content_pointers,
            "Max content pointers should match"
        );
        assert_eq!(
            block.max_linked_content_pointers, block2.max_linked_content_pointers,
            "Max linked content pointers should match"
        );
        assert_eq!(block.signature, block2.signature, "Signature should match");
    }
}
