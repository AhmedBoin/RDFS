//! # RDFS InodeBlock Module
//!
//! This module defines the **Inode** data structures for the RDFS (Redundant Distributed File System).
//! Inodes serve as the metadata hubs for both files and directories within the filesystem. They track
//! critical properties like creation time, size, block allocation, and linkage between inodes.
//!
//! ## Primary Structures
//! - [`InodeDir`] and [`InodeFile`]: Core structures for directories and files
//! - [`InodeLinkedDir`] and [`InodeLinkedFile`]: Extension blocks for large directories/files
//! - [`DirContent`] and [`FileContent`]: Block pointers for directory entries and file content
//! - [`ContentName`]: UTF-32 compatible name container supporting multilingual/emoji-safe storage
//!
//! ## Features
//! - **UTF-32 Naming** for full internationalization (`ContentName`)
//! - **Spacetime Verifiability** using `timestamp` and `signature`
//! - **Linked Expansion** to overcome single-block inode limits
//! - **Type-Safe Differentiation** between file and directory pointers via `InodeType`
//!
//! ## Layout Summary
//! ### InodeDir / InodeFile (typical layout: 1136 bytes + content + signature)
//! ```text
//! - ContentName (1024 bytes)
//! - created (8 bytes)
//! - modify (8 bytes)
//! - size (8 bytes)
//! - total_blocks (8 bytes)
//! - linked (8 bytes)
//! - content length (8 bytes)
//! - [Vec<Content>] (N * 16 bytes)
//! - signature (64 bytes)
//! ```
//!
//! ### InodeLinkedDir / InodeLinkedFile (typical layout: 80 + content + signature)
//! ```text
//! - linked (8 bytes)
//! - content length (8 bytes)
//! - [Vec<Content>] (N * 16 bytes)
//! - signature (64 bytes)
//! ```
//!
//! ## Notes
//! - All serialization logic pads to `block_size` and appends a 64-byte `signature`
//! - `ContentName` uses `u32`-based UTF to support non-ASCII characters with cross-platform consistency
//! - `DirContent` uses `inode_type` to differentiate internal references (file vs. directory)
//!
//! ## Security
//! - Signatures are externally attached via `add_signature()`
//! - Block integrity is guaranteed by deterministic layout and fixed encoding structure
//!
//! Copyrights © 2025 RDFS Contributors. All rights reserved.

use super::super::constants::{CONTENT_SIZE, RESERVED_IB, RESERVED_LIB, SIG_SIZE, Signature};
use std::fmt;
use super::super::rdfs_errors::RDFSError;
use anyhow::Result;

/// Represents an inode in the filesystem, which can be a directory.
/// Inodes are used to store metadata about files and directories, such as their names, sizes, timestamps, and content pointers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InodeDir {
    // 1136 bytes
    pub name: ContentName,
    pub created: u64,
    pub modify: u64,
    pub size: u64,
    pub total_blocks: u64,
    pub content: Vec<DirContent>, // (pointer, Inode type)
    pub linked: u64,              // Pointer to the linked directory or file, 0 if not linked
    pub signature: Signature,     // Signature for the inode, used for verification
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InodeLinkedDir {
    // 80 + padding
    pub content: Vec<DirContent>, // (pointer, Inode type)
    pub linked: u64,              // Pointer to the linked directory or file, 0 if not linked
    pub signature: Signature,     // Signature for the inode, used for verification
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirContent {
    pub pointer: u64,
    pub inode_type: InodeType,
}

impl DirContent {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(CONTENT_SIZE);
        data.extend_from_slice(&self.pointer.to_le_bytes());
        data.extend_from_slice(&(self.inode_type as u64).to_le_bytes());
        data
    }

    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            pointer: u64::from_le_bytes(data[..8].try_into().unwrap()),
            inode_type: InodeType::from(u64::from_le_bytes(data[8..].try_into().unwrap())),
        }
    }
}

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeType {
    Dir = 0,  // Directory
    File = 1, // Regular file
}

impl From<u64> for InodeType {
    fn from(value: u64) -> Self {
        match value {
            0 => InodeType::Dir,
            _ => InodeType::File,
        }
    }
}

/// Represents an inode in the filesystem, which can be a file.
/// Inodes are used to store metadata about files and directories, such as their names, sizes, timestamps, and content pointers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InodeFile {
    // 1136 bytes
    pub name: ContentName,
    pub created: u64,
    pub modify: u64,
    pub size: u64,
    pub total_blocks: u64,
    pub content: Vec<FileContent>, // (pointer, size in blocks)
    pub linked: u64,               // Pointer to the linked directory or file, 0 if not linked
    pub signature: Signature,      // Signature for the inode, used for verification
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InodeLinkedFile {
    // 80
    pub content: Vec<FileContent>, // (pointer, size in blocks)
    pub linked: u64,               // Pointer to the linked directory or file, 0 if not linked
    pub signature: Signature,      // Signature for the inode, used for verification
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileContent {
    pub pointer: u64,
    pub blocks: u64,
}

impl FileContent {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(CONTENT_SIZE);
        data.extend_from_slice(&self.pointer.to_le_bytes());
        data.extend_from_slice(&self.blocks.to_le_bytes());
        data
    }

    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            pointer: u64::from_le_bytes(data[..8].try_into().unwrap()),
            blocks: u64::from_le_bytes(data[8..].try_into().unwrap()),
        }
    }
}

/// Any content (directory or file) is named in UTF-32, because English is not the only language used and using UTF-8
/// in other systems scrambling the names of your contents if not named in english, the most suitable solution for this
/// is using 32 bit code for more additional uni codes now you can write in any different language or even uses Emoji 👍.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentName {
    // UTF-32     1024 bytes
    pub length: u32,
    pub name: [u32; 255],
}

impl ContentName {
    pub fn new(s: &str) -> Self {
        let mut name = [0u32; 255];
        let chars: Vec<u32> = s.chars().take(255).map(|c| c as u32).collect();
        for (i, c) in chars.iter().enumerate() {
            name[i] = *c;
        }
        Self {
            length: chars.len() as u32,
            name,
        }
    }

    /// Returns the actual file name as a String
    pub fn as_string(&self) -> String {
        self.name[..(self.length as usize)]
            .iter()
            .map(|&c| char::from_u32(c).unwrap_or('\u{FFFD}')) // safe fallback
            .collect()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1024);
        buf.extend(&self.length.to_le_bytes());
        for &c in self.name.iter() {
            buf.extend(c.to_le_bytes());
        }
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Self {
        let length = u32::from_le_bytes(data[..4].try_into().unwrap());
        let mut name = [0u32; 255];
        for (i, item) in name.iter_mut().enumerate() {
            let start = (i + 1) * 4;
            let bytes: [u8; 4] = data[start..start + 4].try_into().unwrap();
            *item = u32::from_le_bytes(bytes);
        }

        Self { length, name }
    }
}

impl fmt::Display for ContentName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name: String = self.name[..(self.length as usize)]
            .iter()
            .map(|&c| char::from_u32(c).unwrap_or('\u{FFFD}'))
            .collect();
        write!(f, "{name}")
    }
}

impl InodeDir {
    pub fn new(name: ContentName, timestamp: u64, size: u64, total_blocks: u64, content: Vec<DirContent>, linked: u64) -> Self {
        Self {
            name,
            created: timestamp,
            modify: timestamp,
            size,
            total_blocks,
            content,
            linked,
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

        encoded.extend_from_slice(&self.name.to_bytes());
        encoded.extend_from_slice(&self.created.to_le_bytes());
        encoded.extend_from_slice(&self.modify.to_le_bytes());
        encoded.extend_from_slice(&self.size.to_le_bytes());
        encoded.extend_from_slice(&self.total_blocks.to_le_bytes());
        encoded.extend_from_slice(&self.linked.to_le_bytes());
        encoded.extend_from_slice(&(self.content.len() as u64).to_le_bytes());
        for content in self.content.iter() {
            encoded.extend_from_slice(&content.to_bytes());
        }
        encoded.resize(block_size - SIG_SIZE, 0);
        encoded.extend_from_slice(&self.signature);

        encoded
    }

    pub fn from_bytes(data: &[u8], block_size: usize) -> Result<Self> {
        if data.len() != block_size {
            return Err(RDFSError::InvalidInodeBlockLength.into());
        }

        let name = ContentName::from_bytes(&data[..1024]);
        let created = u64::from_le_bytes(data[1024..1032].try_into().unwrap());
        let modify = u64::from_le_bytes(data[1032..1040].try_into().unwrap());
        let size = u64::from_le_bytes(data[1040..1048].try_into().unwrap());
        let total_blocks = u64::from_le_bytes(data[1048..1056].try_into().unwrap());
        let linked = u64::from_le_bytes(data[1056..1064].try_into().unwrap());

        let length = u64::from_le_bytes(data[1064..1072].try_into().unwrap()) as usize;
        if length > block_size - RESERVED_IB {
            return Err(RDFSError::InvalidEncodedInodeBlockLength.into());
        }

        let mut content = Vec::with_capacity(length);
        for i in 0..length {
            let start = 1072 + (i * CONTENT_SIZE);
            content.push(DirContent::from_bytes(&data[start..start + CONTENT_SIZE]));
        }
        let signature: Signature = data[block_size - SIG_SIZE..].try_into().unwrap();

        Ok(Self {
            name,
            created,
            modify,
            size,
            total_blocks,
            content,
            linked,
            signature,
        })
    }
}

impl InodeLinkedDir {
    pub fn new(content: Vec<DirContent>, linked: u64) -> Self {
        Self {
            content,
            linked,
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

        encoded.extend_from_slice(&self.linked.to_le_bytes());
        encoded.extend_from_slice(&(self.content.len() as u64).to_le_bytes());
        for content in self.content.iter() {
            encoded.extend_from_slice(&content.to_bytes());
        }
        encoded.resize(block_size - SIG_SIZE, 0);
        encoded.extend_from_slice(&self.signature);

        encoded
    }

    pub fn from_bytes(data: &[u8], block_size: usize) -> Result<Self> {
        if data.len() != block_size {
            return Err(RDFSError::InvalidInodeBlockLength.into());
        }
        let linked = u64::from_le_bytes(data[..8].try_into().unwrap());

        let length = u64::from_le_bytes(data[8..16].try_into().unwrap()) as usize;
        if length > block_size - RESERVED_LIB {
            return Err(RDFSError::InvalidEncodedInodeBlockLength.into());
        }

        let mut content = Vec::with_capacity(length);
        for i in 0..length {
            let start = 16 + (i * CONTENT_SIZE);
            content.push(DirContent::from_bytes(&data[start..start + CONTENT_SIZE]));
        }
        let signature: Signature = data[block_size - SIG_SIZE..].try_into().unwrap();

        Ok(Self { content, linked, signature })
    }
}

impl InodeFile {
    pub fn new(name: ContentName, timestamp: u64, size: u64, total_blocks: u64, content: Vec<FileContent>, linked: u64) -> Self {
        Self {
            name,
            created: timestamp,
            modify: timestamp,
            size,
            total_blocks,
            content,
            linked,
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

        encoded.extend_from_slice(&self.name.to_bytes());
        encoded.extend_from_slice(&self.created.to_le_bytes());
        encoded.extend_from_slice(&self.modify.to_le_bytes());
        encoded.extend_from_slice(&self.size.to_le_bytes());
        encoded.extend_from_slice(&self.total_blocks.to_le_bytes());
        encoded.extend_from_slice(&self.linked.to_le_bytes());
        encoded.extend_from_slice(&(self.content.len() as u64).to_le_bytes());
        for content in self.content.iter() {
            encoded.extend_from_slice(&content.to_bytes());
        }
        encoded.resize(block_size - SIG_SIZE, 0);
        encoded.extend_from_slice(&self.signature);

        encoded
    }

    pub fn from_bytes(data: &[u8], block_size: usize) -> Result<Self> {
        if data.len() != block_size {
            return Err(RDFSError::InvalidInodeBlockLength.into());
        }

        let name = ContentName::from_bytes(&data[..1024]);
        let created = u64::from_le_bytes(data[1024..1032].try_into().unwrap());
        let modify = u64::from_le_bytes(data[1032..1040].try_into().unwrap());
        let size = u64::from_le_bytes(data[1040..1048].try_into().unwrap());
        let total_blocks = u64::from_le_bytes(data[1048..1056].try_into().unwrap());
        let linked = u64::from_le_bytes(data[1056..1064].try_into().unwrap());

        let length = u64::from_le_bytes(data[1064..1072].try_into().unwrap()) as usize;
        if length > block_size - RESERVED_IB {
            return Err(RDFSError::InvalidEncodedInodeBlockLength.into());
        }

        let mut content = Vec::with_capacity(length);
        for i in 0..length {
            let start = 1072 + (i * CONTENT_SIZE);
            content.push(FileContent::from_bytes(&data[start..start + CONTENT_SIZE]));
        }
        let signature: Signature = data[block_size - SIG_SIZE..].try_into().unwrap();

        Ok(Self {
            name,
            created,
            modify,
            size,
            total_blocks,
            content,
            linked,
            signature,
        })
    }
}

impl InodeLinkedFile {
    pub fn new(content: Vec<FileContent>, linked: u64) -> Self {
        Self {
            content,
            linked,
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

        encoded.extend_from_slice(&self.linked.to_le_bytes());
        encoded.extend_from_slice(&(self.content.len() as u64).to_le_bytes());
        for content in self.content.iter() {
            encoded.extend_from_slice(&content.to_bytes());
        }
        encoded.resize(block_size - SIG_SIZE, 0);
        encoded.extend_from_slice(&self.signature);

        encoded
    }

    pub fn from_bytes(data: &[u8], block_size: usize) -> Result<Self> {
        if data.len() != block_size {
            return Err(RDFSError::InvalidInodeBlockLength.into());
        }
        let linked = u64::from_le_bytes(data[..8].try_into().unwrap());

        let length = u64::from_le_bytes(data[8..16].try_into().unwrap()) as usize;
        if length > block_size - RESERVED_LIB {
            return Err(RDFSError::InvalidEncodedInodeBlockLength.into());
        }

        let mut content = Vec::with_capacity(length);
        for i in 0..length {
            let start = 16 + (i * CONTENT_SIZE);
            content.push(FileContent::from_bytes(&data[start..start + CONTENT_SIZE]));
        }
        let signature: Signature = data[block_size - SIG_SIZE..].try_into().unwrap();

        Ok(Self { content, linked, signature })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_inode() {
        let block_size = 4096;
        let file_name = ContentName::new("test_file.txt");
        let content = DirContent {
            pointer: 3,
            inode_type: InodeType::Dir,
        };
        let mut inode = InodeDir::new(file_name.clone(), 7, 11, 1, vec![content.clone(), content], 0);
        inode.add_signature([255; 64]);

        // Serialize the inode
        let serialized = inode.to_bytes(block_size);
        println!("Serialized Inode: {:?}", serialized.len());
        // println!("Data: {:?}", serialized);

        // Deserialize back to an inode
        let deserialized = InodeDir::from_bytes(&serialized, block_size).unwrap();

        // Check if the original and deserialized inodes are equal
        assert_eq!(inode.name.to_string(), deserialized.name.to_string());
        assert_eq!(inode.created, deserialized.created);
        assert_eq!(inode.modify, deserialized.modify);
        assert_eq!(inode.size, deserialized.size);
        assert_eq!(inode.total_blocks, deserialized.total_blocks);
        assert_eq!(inode.content, deserialized.content);
        assert_eq!(inode.linked, deserialized.linked);
        assert_eq!(inode.signature, deserialized.signature);
    }

    #[test]
    fn test_inode2() {
        let block_size = 4096;
        let file_name = ContentName::new("test_file.txt");
        let content = FileContent { pointer: 3, blocks: 10 };
        let mut inode = InodeFile::new(file_name.clone(), 7, 11, 1, vec![content.clone(), content], 0);
        inode.add_signature([255; 64]);

        // Serialize the inode
        let serialized = inode.to_bytes(block_size);
        println!("Serialized Inode: {:?}", serialized.len());
        // println!("Data: {:?}", serialized);

        // Deserialize back to an inode
        let deserialized = InodeFile::from_bytes(&serialized, block_size).unwrap();

        // Check if the original and deserialized inodes are equal
        assert_eq!(inode.name.to_string(), deserialized.name.to_string());
        assert_eq!(inode.created, deserialized.created);
        assert_eq!(inode.modify, deserialized.modify);
        assert_eq!(inode.size, deserialized.size);
        assert_eq!(inode.total_blocks, deserialized.total_blocks);
        assert_eq!(inode.content, deserialized.content);
        assert_eq!(inode.linked, deserialized.linked);
        assert_eq!(inode.signature, deserialized.signature);
    }

    #[test]
    fn test_linked_inode() {
        let block_size = 4096;
        let linked_inode = InodeLinkedDir::new(vec![], 0);

        // Serialize the linked inode
        let serialized = linked_inode.to_bytes(block_size);
        println!("Serialized LinkedInode: {:?}", serialized.len());

        // Deserialize back to a linked inode
        let deserialized = InodeLinkedDir::from_bytes(&serialized, block_size).unwrap();

        // Check if the original and deserialized linked inodes are equal
        assert_eq!(linked_inode.content, deserialized.content);
        assert_eq!(linked_inode.linked, deserialized.linked);
        assert_eq!(linked_inode.signature, deserialized.signature);
    }
}
