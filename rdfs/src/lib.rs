//! # Rust Distributed File System (RDFS)
//!
//! A simple distributed file system written in Rust.
//!
//! The main advantage of this type of virtual file system is
//!     1. zero copy/move contents (like delete in any file system).
//!     2. no fragmentation due to semi-linked list structure (linked list for arrays of data blocks).
//!     3. continues linked drivers as a one big driver.
//!     4. high throughput and low latency R times where R is the redundancy where you could
//!         receive file chunks in different nodes in parallel.
//!     5. resistance to cyber attacks like DoS/DDoS...etc.
//!     6. unstoppable due to it's nature of distribution across different nodes also this nodes
//!         could be operate under NATs, Relays or TorVPN.
//!
//! Copyrights © 2025, RDFS Contributors
#![feature(core_float_math)]

pub mod config;
pub mod constants;
pub mod core;
pub mod file_system;
pub mod prelude;
pub mod rdfs_errors;
pub mod utils;

pub mod client;
pub mod server;
