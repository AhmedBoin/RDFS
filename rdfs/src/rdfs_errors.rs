//! # RDFS Error Definitions Module
//!
//! This module defines all the structured errors used across the RDFS (Redundant Distributed File System).
//! It centralizes the various error types into a single enum (`RDFSError`) to allow meaningful and descriptive
//! propagation of issues encountered during file system operations.
//!
//! ## Purpose
//! - Provide granular and human-readable error variants for all critical failure cases
//! - Simplify debugging and tracing through descriptive error messages
//! - Support interoperability with `anyhow::Result` and standard error chaining
//!
//! ## Example
//! ```rust
//! use rdfs::RDFSError;
//!
//! fn validate_magic_word(word: &[u8]) -> Result<(), RDFSError> {
//!     if word != b"RDFS-SHR" && word != b"RDFS-PRV" {
//!         return Err(RDFSError::InvalidMagicWord);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Error Categories
//! - **Structural Errors**: super block, address block, bitmaps, inodes, block size
//! - **Semantic Errors**: invalid content sizes, alignment issues, logical misuse
//! - **System Constraints**: pointer boundary checks, unsupported operations in mode (e.g., bitmaps in private RDFS)
//!
//! These errors are meant to protect data integrity and catch misuse of the RDFS API at runtime.
//!
//! Copyrights © 2025 RDFS Contributors. All rights reserved.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RDFSError {
    #[error("Invalid super block length")]
    InvalidSuperBlockLength,

    #[error("Invalid magic word")]
    InvalidMagicWord,

    #[error("Input length not equal nodes address size")]
    InvalidAddressBlockLength,

    #[error("Encoded length not equal nodes address size")]
    InvalidEncodedAddressBlockLength,

    #[error("Input length not equal bitmaps size")]
    InvalidBitmapsBlockLength,

    #[error("encoded length not equal bitmaps size")]
    InvalidEncodedBitmapsBlockLength,

    #[error("Input length not equal block size")]
    InvalidDataBlockLength,

    #[error("content length is greater than block size")]
    InvalidEncodedDataBlockLength,

    #[error("Input length not equal block size")]
    InvalidInodeBlockLength,

    #[error("content length is greater than block size")]
    InvalidEncodedInodeBlockLength,

    #[error("No bitmaps in private RDFS")]
    NoBitmapsPrivateRDFS,

    #[error("No bitmaps in private RDFS")]
    InvalidPointerAlignment,

    #[error("pointer is less or greater than actual data pointer")]
    PointerOutOfRange,
}
