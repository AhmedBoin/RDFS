//! # RDFS Constants Module
//!
//! This file contains core constants used throughout various RDFS structs.
//! The design emphasizes modularity, allowing for future schema changes or
//! cryptographic upgrades with minimal disruption.
//!
//! In particular, cryptographic primitives are isolated here so they can be
//! swapped out easily—e.g., for migrating to a post-quantum secure signature scheme.
//!
//! ## Purpose
//! - Improve maintainability and readability
//! - Centralize schema definitions
//! - Facilitate future extensibility
//!
//! Copyrights © 2025 RDFS Contributors. All rights reserved.

pub const PK_SIZE: usize = 32;
pub const SK_SIZE: usize = 32;
pub const SIG_SIZE: usize = 64;

pub const SB_SIZE: usize = 16 * 8 + PK_SIZE + PK_SIZE + SIG_SIZE;
pub const RESERVED_AB: usize = 72;
pub const RESERVED_BB: usize = 96;
pub const RESERVED_DB: usize = 88;
pub const RESERVED_CDB: usize = 92; // -> additional 4 bytes for client due to RaptorQ code encoding
pub const RESERVED_IB: usize = 1136;
pub const RESERVED_LIB: usize = 80;

pub const CONTENT_SIZE: usize = 16; // (pointer, type) or (pointer, size)

pub const FS_MAGIC_SHARED: u64 = u64::from_le_bytes(*b"RDFS-SHR");
pub const FS_MAGIC_PRIVATE: u64 = u64::from_le_bytes(*b"RDFS-PRV");

pub type Address = [u8; PK_SIZE];
pub type Signature = [u8; SIG_SIZE];
