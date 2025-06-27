//! # RDFS File System Module
//!
//! This module implements the core logic for creating, mounting, reading, and writing
//! to RDFS (Redundant Distributed File System) virtual drives. It encapsulates the
//! functionality to initialize and manipulate both shared and private file system types.
//!
//! ## Key Responsibilities
//! - Create and initialize a new RDFS file with structured layout (superblock, address block, etc.)
//! - Mount an existing RDFS drive by reading its metadata
//! - Support reading and writing to file blocks and metadata regions
//! - Ensure alignment and boundary correctness of all operations
//! - Allow streaming large file content block-by-block with minimal memory usage
//!
//! ## Design Principles
//! - Safe low-level access to file-mapped regions with alignment and range checks
//! - Modular separation of shared/private file system initialization
//! - Integrates cryptographic and content metadata abstractions from core modules
//!
//! ## File Layout (Abstracted)
//! ```text
//! [SuperBlock | NodesAddresses | BitmapsBlock (shared only) | DataBlocks... | InodeRoot]
//! ```
//!
//! This structure allows deterministic offsets and fast access, while maintaining
//! modular encoding of components.
//!
//! ## Usage
//! Mount a drive and read a block:
//! ```rust
//! let fs = RDFS::mount_drive("data/example.RDFS")?;
//! let root_inode_block = fs.read_block(fs.system.inode_pointer)?;
//! ```
//!
//! ## Compatibility
//! Supports:
//! - Shared RDFS: includes bitmaps and hierarchical inode tree
//! - Private RDFS: excludes bitmaps, minimal metadata
//!
//! Copyrights © 2025 RDFS Contributors. All rights reserved.

#![allow(clippy::too_many_arguments)]
use std::fmt::Debug;
use std::path::{Path, PathBuf};

use crate::core::super_block::FileSystemType;

use crate::core::addresses_block::AddressesBlock;
use crate::core::bitmaps_block::BitmapsBlock;
use crate::core::inode_block::{ContentName, FileContent, InodeDir};
use crate::core::super_block::SuperBlock;
use crate::utils::{bytes_to_hex, create_physical_file, current_time_as_u64, read_range, write_range};

use super::constants::{Address, PK_SIZE, SIG_SIZE};
use super::rdfs_errors::RDFSError;
use anyhow::Result;


#[derive(Debug, Clone)]
pub struct RDFS {
    pub path: PathBuf,
    pub system: SuperBlock,
}

impl RDFS {
    pub fn new<P: AsRef<Path>>(
        path: P,
        magic: FileSystemType,
        owner: Address,
        program_id: Address,
        storage: u64,
        redundancy: u64,
        nodes: u64,
        block_size: u64,
    ) -> Result<Self> {
        match magic {
            FileSystemType::Shared => Self::new_shared(path, magic, owner, program_id, storage, redundancy, nodes, block_size),
            FileSystemType::Private => Self::new_private(path, magic, owner, program_id, storage, redundancy, nodes, block_size),
        }
    }

    /// Creates a new shared RDFS object with the given parameters.
    pub fn new_shared<P: AsRef<Path>>(
        path: P,
        magic: FileSystemType,
        owner: Address,
        program_id: Address,
        storage: u64,
        redundancy: u64,
        nodes: u64,
        block_size: u64,
    ) -> Result<Self> {
        // Create the super block with the provided parameters
        let timestamp = current_time_as_u64()?;
        let super_block = SuperBlock::new(magic, owner, program_id, storage, redundancy, nodes, block_size);
        let addresses_block = AddressesBlock::new(vec![[0; PK_SIZE]; super_block.nodes as usize], [0; SIG_SIZE]);
        let mut bitmaps_block = BitmapsBlock::new(super_block.total_blocks, timestamp);
        let root_inode = InodeDir::new(ContentName::new("./"), timestamp, 0, super_block.total_blocks, vec![], 0);
        bitmaps_block.set_bit(super_block.total_blocks as usize - 1); // Set the last block for root inode

        // Create the file name based on the program ID
        let path = Path::new(path.as_ref()).join(&(bytes_to_hex(&program_id) + ".RDFS"));
        let size = super_block.node_storage;

        create_physical_file(&path, size)?;
        write_range(&path, 0, &super_block.to_bytes())?;
        write_range(&path, super_block.nodes_address_pointer, &addresses_block.to_bytes())?;
        write_range(&path, super_block.bitmaps_pointer, &bitmaps_block.to_bytes())?;
        write_range(&path, super_block.inode_pointer, &root_inode.to_bytes(super_block.block_size as usize))?;

        let rdfs = Self { path, system: super_block };

        Ok(rdfs)
    }

    /// Creates a new private RDFS object with the given parameters.
    pub fn new_private<P: AsRef<Path>>(
        path: P,
        magic: FileSystemType,
        owner: Address,
        program_id: Address,
        storage: u64,
        redundancy: u64,
        nodes: u64,
        block_size: u64,
    ) -> Result<Self> {
        // Create the super block with the provided parameters
        let super_block = SuperBlock::new(magic, owner, program_id, storage, redundancy, nodes, block_size);
        let addresses_block = AddressesBlock::new(vec![[0; PK_SIZE]; super_block.nodes as usize], [0; SIG_SIZE]);

        // Create the file name based on the program ID
        let path = Path::new(path.as_ref()).join(&(bytes_to_hex(&program_id) + ".RDFS"));
        let size = super_block.node_storage;

        create_physical_file(&path, size)?;
        write_range(&path, 0, &super_block.to_bytes())?;
        write_range(&path, super_block.nodes_address_pointer, &addresses_block.to_bytes())?;

        let rdfs = Self { path, system: super_block };

        Ok(rdfs)
    }

    pub fn mount_drive<P: AsRef<Path>>(path: P) -> Result<Self> {
        let super_block = SuperBlock::from_bytes(&read_range(&path, 0, 256)?)?;
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            system: super_block,
        })
    }

    pub fn unmount_drive(self) -> Result<()> {
        //! In this implementation, unmounting does not require any specific action.
        //! However, maybe we will implement some necessary cleanup or finalization here.
        Ok(())
    }

    pub fn read_super_block(&self) -> Vec<u8> {
        self.system.to_bytes()
    }

    pub fn read_nodes_addresses(&self) -> Result<Vec<u8>> {
        let start = self.system.nodes_address_pointer;
        let end = start + self.system.nodes_address_size;

        read_range(&self.path, start, end)
    }

    /// used only in shared RDFS, using in private RDFS return an Error.
    pub fn read_bitmaps(&self) -> Result<Vec<u8>> {
        match &self.system.magic {
            FileSystemType::Shared => {
                let start = self.system.bitmaps_pointer;
                let end = start + self.system.bitmaps_size;

                read_range(&self.path, start, end)
            }
            FileSystemType::Private => Err(RDFSError::NoBitmapsPrivateRDFS.into()),
        }
    }

    /// Reads a block of data from the file system at the specified pointer.
    /// The pointer is the starting position of the block in the file.
    /// Returns the block data as a `Vec<u8>`.
    /// it could be used in shared RDFS for retrieving specific `Inode`
    /// or specific block in private RDFS
    pub fn read_block(&self, pointer: u64) -> Result<Vec<u8>> {
        if pointer < self.system.data_pointer {
            return Err(RDFSError::PointerOutOfRange.into());
        }
        if (pointer - self.system.data_pointer) % self.system.block_size != 0 {
            return Err(RDFSError::InvalidPointerAlignment.into());
        }
        let start = pointer;
        let end = pointer + self.system.block_size;
        read_range(&self.path, start, end)
    }

    /// Reads multiple blocks from the file system based on the provided ranges.
    /// Each range specifies a starting pointer and the number of blocks to read.
    /// Returns an iterator over the read blocks as `Vec<u8>`.
    /// It has been designed in this way because the total requested data block
    /// will be much more larger than our memory, so you can iter on these blocks,
    /// read it one by one and send it over network.
    pub fn read_blocks(&self, ranges: Vec<FileContent>) -> Box<dyn Iterator<Item = Vec<u8>>> {
        let path = self.path.clone();
        let block_size = self.system.block_size;

        let iter = ranges
            .into_iter()
            .flat_map(move |content| {
                let path = path.clone(); // clone for move into closure
                (0..content.blocks).map(move |block| {
                    let start = content.pointer + block * block_size;
                    let end = start + block_size;
                    read_range(&path, start, end)
                })
            })
            .filter_map(Result::ok);

        Box::new(iter)
    }

    /// Updates the addresses block with the provided block.
    pub fn write_nodes_addresses(&self, data: &[u8]) -> Result<()> {
        let address = AddressesBlock::from_bytes(data, self.system.nodes_address_size as usize)?;
        if address.addresses.len() != self.system.nodes as usize {
            return Err(RDFSError::InvalidAddressBlockLength.into());
        }

        write_range(&self.path, self.system.nodes_address_pointer, data)
    }

    /// Update the bitmaps block with the provided block.
    pub fn write_bitmaps(&self, data: &[u8]) -> Result<()> {
        match self.system.magic {
            FileSystemType::Shared => {
                let bitmaps = BitmapsBlock::from_bytes(data, self.system.bitmaps_size as usize)?;
                if (bitmaps.total_blocks != self.system.total_blocks) | (bitmaps.bit_field.len() != self.system.total_blocks as usize) {
                    return Err(RDFSError::InvalidBitmapsBlockLength.into());
                }

                write_range(&self.path, self.system.bitmaps_pointer, data)
            }
            FileSystemType::Private => Err(RDFSError::NoBitmapsPrivateRDFS.into()),
        }
    }

    pub fn write_block(&self, pointer: u64, data: &[u8]) -> Result<()> {
        if pointer < self.system.data_pointer {
            return Err(RDFSError::PointerOutOfRange.into());
        }
        if (pointer - self.system.data_pointer) % self.system.block_size != 0 {
            return Err(RDFSError::InvalidPointerAlignment.into());
        }
        write_range(&self.path, pointer, data)
    }
}
