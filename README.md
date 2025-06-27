# RDFS — RaptorQ Distributed File System

**RDFS** (RaptorQ Distributed File System) is a decentralized, fault-tolerant virtual file system built entirely in **Rust**. Unlike traditional distributed hash tables (DHTs) or fully replicated systems, RDFS uses *erasure coding* (specifically RaptorQ) to fragment data into *independent, recoverable blocks* distributed across multiple nodes—minimizing redundancy and maximizing availability.

> “Instead of every node storing everything, RDFS ensures each node stores *just enough* to collectively rebuild everything.”

---

## 🚀 Overview

Traditional systems often **replicate entire files or chunks across many nodes**, consuming significant storage. RDFS breaks away from this by **distributing unique encoded data fragments to each node**, with only a **minimal redundancy factor**. This allows full file recovery even if a large portion of the network becomes unavailable.

---

## 🔧 Core Design Principles

- **Data is encoded, not blindly replicated**  
  RDFS uses RaptorQ erasure coding to encode data with slight redundancy (e.g., 1.3× instead of 3× replication). This allows file recovery as long as a threshold number of fragments are retrieved.

- **Efficient and fair storage utilization**  
  In a 1000-node system storing 1 TB of data with 1× redundancy, each node only needs to store ~2 GB. This is *massively scalable* and avoids burdening individual nodes.

- **Fault tolerance**  
  Even if **50% of nodes go offline**, full data recovery is guaranteed—as long as the remaining fragments meet the decoding threshold.

- **Limitless encoding flexibility**  
  RDFS can generate *new encoded blocks on-demand* to accommodate new nodes joining the network. It also supports **re-encoding or redistribution** for optimized chunk sizing and balanced network load.

- **Virtual filesystem abstraction**  
  RDFS presents a file system–like abstraction over distributed data, enabling logical directories, inodes, bitmaps, and data blocks. All structures are self-describing, verifiable, and serialized.

---

## 📁 File System Layout (Virtualized)

RDFS includes a virtual file system layer, representing structured metadata and layout logic:

- **SuperBlock** — root metadata: capacity, block size, pointers to other structures
- **AddressesBlock** — list of participating node addresses
- **BitmapsBlock** — allocation maps for used/free blocks
- **DataBlock** — content-carrying block with timestamp and unique ID
- **InodeBlock** — file/directory metadata and pointers (supports nested and linked structures)

Each block includes a **signature field** for integrity and authentication. Every structure implements precise byte-level serialization/deserialization for network transport or persistent storage.

---

## 📦 Advantages of RDFS

- **No central coordination or consensus** is required to fetch or rebuild data.
- **Compact per-node storage**, even for large-scale datasets.
- **Self-healing**: new nodes can be seeded with fresh redundant blocks on the fly.
- **Reliable virtual file system** supports real-world directory and file abstractions.
- **Written in Rust** — ensuring safety, concurrency, and performance.

---

## 📊 Use Case Example

> Storing 1 TB of data with 1× redundancy in a 1000-node cluster:

- **Encoded size**: 2 TB (1 TB original + 1 TB redundancy)
- **Data per node**: 2 GB
- **Recovery threshold**: any ~50% of nodes
- **Uniqueness**: each node stores a *unique slice* — no duplication

In contrast, a naive replication system storing the same data across 1000 nodes would require **1 PB** (1,000 TB) of total storage — a 500× increase over RDFS.

---

## ❌ Why Not Replication?

- Replication = wasted space & tight coupling.
- If two nodes storing the same chunk both go offline → **data loss**.
- In RDFS, any *arbitrary set* of blocks can be used to restore the data — not tied to specific node combinations.

---

## 📚 Codebase Highlights

- 100% **Rust** implementation
- Modular and well-organized:
  - `super_block.rs`, `addresses_block.rs`, `bitmaps_block.rs`
  - `data_block.rs`, `inode_block.rs`
  - `rdfs_errors.rs` for custom error handling
- Efficient binary serialization and deserialization
- All structures include `to_bytes` / `from_bytes`
- UTF-32 support for international and emoji-compatible filenames

---

## ✅ Status

| Feature                   | Status |
|---------------------------|--------|
| Virtual file system       | ✅ Done |
| Erasure-coded blocks      | ✅ Done |
| Inode structures          | ✅ Done |
| Bitmaps and pointers      | ✅ Done |
| Signature placeholders    | ✅ Done |
| Distributed sync/network  | ⏳ Planned |
| CLI tools or API          | ⏳ Planned |
| Dynamic rebalancing       | ⏳ Planned |

---

## 📎 License

**MIT OR Apache-2.0**

You can freely use, modify, and redistribute this project under either license.

---

## 🦀 Built in Rust

This project is fully written in Rust, using modern features for performance and safety. Rust’s ecosystem enabled rapid and safe development of a performant, concurrent file system abstraction that compiles to efficient native binaries.

---

## 🙌 Contributing

Contributions are welcome! If you're interested in:

- Erasure coding
- Distributed storage
- Rust-based infrastructure
- File system design

Feel free to open issues, submit PRs, or fork the project.

---

## 🌐 Contact

Made with 🦀 and passion by [@AhmedBoin](https://github.com/AhmedBoin)  
Copyright © 2025

For questions, collaborations, or feedback, feel free to reach out:

• 📧 Gmail: [Ahmed.Boin@gmail.com]

• 💼 LinkedIn: [https://www.linkedin.com/in/ahmed-boin/]

• 🐦 Twitter: [https://x.com/AhmedBoin]

---