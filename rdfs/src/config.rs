//! # RDFS Configuration Module
//!
//! This module defines and manages persistent configuration settings for RDFS,
//! including the working directory and candidate storage paths with their available space.
//!
//! The configuration is stored as a TOML file (`RDFSConfig.toml`) and provides
//! mechanisms for loading, saving, and querying space-aware paths.
//!
//! ## Features
//! - Serialize and deserialize paths with associated available space
//! - Dynamically determine disk space availability using `sysinfo`
//! - Add, remove, and query storage paths based on space requirements
//! - Designed for persistence across application runs
//!
//! ## Design Goals
//! - Portability across environments
//! - Minimal and safe disk access logic
//! - Easy extensibility for future configuration needs
//!
//! Copyrights © 2025 RDFS Contributors. All rights reserved.

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use sysinfo::Disks;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RDFSConfig {
    pub currant_path: Option<RDFSPath>,
    pub search_paths: Vec<RDFSPath>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RDFSPath {
    pub path: PathBuf,
    pub available: u64,
}

impl RDFSConfig {
    pub fn load() -> std::io::Result<RDFSConfig> {
        read_toml_file("RDFSConfig.toml")
    }

    pub fn save(&self) -> std::io::Result<()> {
        write_toml_file("RDFSConfig.toml", self)
    }

    pub fn add_path<P: AsRef<Path>>(&mut self, path: P) {
        let path_buf = path.as_ref().to_path_buf();
        let available = get_free_space(&path_buf).unwrap_or(0);
        self.search_paths.push(RDFSPath { path: path_buf, available });
    }

    pub fn remove_path<P: AsRef<Path>>(&mut self, path: P) -> bool {
        let path_ref = path.as_ref();
        let original_len = self.search_paths.len();

        self.search_paths.retain(|p| p.path != path_ref);

        // Return true if something was removed
        original_len != self.search_paths.len()
    }

    /// Finds a path with at least `min_space` bytes available.
    /// Returns `Some(&Path)` if one is found, otherwise `None`.
    pub fn get_path_with_space(&self, min_space: u64) -> Option<&Path> {
        self.search_paths.iter().find(|p| p.available >= min_space).map(|p| p.path.as_path())
    }
}

fn get_free_space(path: &Path) -> Option<u64> {
    let disks = Disks::new_with_refreshed_list();

    for disk in disks.iter() {
        if path.starts_with(disk.mount_point()) {
            return Some(disk.available_space());
        }
    }

    None
}

fn write_toml_file(path: &str, config: &RDFSConfig) -> std::io::Result<()> {
    let toml_string = toml::to_string(config).expect("Failed to serialize to TOML");
    let mut file = File::create(path)?;
    file.write_all(toml_string.as_bytes())?;
    Ok(())
}

fn read_toml_file(path: &str) -> std::io::Result<RDFSConfig> {
    let contents = fs::read_to_string(path)?;
    let config: RDFSConfig = toml::from_str(&contents).expect("Failed to parse TOML");
    Ok(config)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env;

    #[test]
    fn test_rdfs_config_save_and_load() {
        let current_dir = env::current_dir().expect("Failed to get current directory");

        let mut config = RDFSConfig::default();
        config.add_path(&current_dir);
        config.currant_path = Some(RDFSPath {
            available: get_free_space(&current_dir).unwrap_or(0),
            path: current_dir.clone(),
        });

        config.save().expect("Failed to save config");

        let loaded = RDFSConfig::load().expect("Failed to load config");

        assert_eq!(loaded.currant_path.as_ref().unwrap().path, current_dir);
        assert!(loaded.search_paths.iter().any(|p| p.path == current_dir));
    }

    #[test]
    fn test_add_and_remove_path() {
        let current_dir = env::current_dir().unwrap();

        let mut config = RDFSConfig::default();
        config.add_path(&current_dir);

        assert!(config.search_paths.iter().any(|p| p.path == current_dir));

        let removed = config.remove_path(&current_dir);
        assert!(removed);
        assert!(!config.search_paths.iter().any(|p| p.path == current_dir));
    }

    #[test]
    fn test_get_path_with_space_found() {
        let current_dir = env::current_dir().unwrap();
        let space = get_free_space(&current_dir).unwrap_or(0);

        let mut config = RDFSConfig::default();
        config.search_paths.push(RDFSPath {
            path: current_dir.clone(),
            available: space,
        });

        let result = config.get_path_with_space(space.saturating_sub(1));
        assert_eq!(result, Some(current_dir.as_path()));
    }

    #[test]
    fn test_get_path_with_space_not_found() {
        let current_dir = env::current_dir().unwrap();

        let mut config = RDFSConfig::default();
        config.search_paths.push(RDFSPath {
            path: current_dir,
            available: 1_000, // small space
        });

        let result = config.get_path_with_space(10_000_000_000); // 10 GB
        assert!(result.is_none());
    }
}
