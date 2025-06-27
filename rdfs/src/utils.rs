use anyhow::{Result, anyhow};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Convert array slice to hex string
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Create a file with given byte size
pub fn create_physical_file<P: AsRef<Path>>(path: P, size: u64) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(false) // Truncate to zero length if it exists
        .write(true) // Create if it doesn’t exist
        .open(path)?;

    // Seek to size - 1, then write 1 byte.
    // This forces allocation of all blocks without writing full content.
    if size > 0 {
        file.seek(SeekFrom::Start(size - 1))?;
        file.write_all(&[0])?;
    }

    Ok(())
}

/// Reads a specific range of bytes from a file.
/// The range is defined by the start and end byte positions.
pub fn read_range<P: AsRef<Path>>(path: P, start: u64, end: u64) -> Result<Vec<u8>> {
    let mut file = File::open(path)?;

    // Move the file cursor to the start byte
    file.seek(SeekFrom::Start(start))?;

    // Calculate how many bytes to read
    let length = end - start;
    let mut buffer = vec![0u8; length as usize];

    // Read exactly that range
    file.read_exact(&mut buffer)?;

    Ok(buffer)
}

/// Writes data to a specific range in a file, starting at `start`.
/// The `data` length determines how many bytes are written.
/// If the file is too short, it will be extended.
pub fn write_range<P: AsRef<Path>>(path: P, start: u64, data: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).open(path)?;

    // Move the file cursor to the start position
    file.seek(SeekFrom::Start(start))?;

    // Write the provided data
    file.write_all(data)?;

    Ok(())
}

/// Returns the current time as a u64 timestamp in seconds since the UNIX epoch.
pub fn current_time_as_u64() -> Result<u64> {
    if let Ok(time) = SystemTime::now().duration_since(UNIX_EPOCH) {
        return Ok(time.as_secs());
    }
    Err(anyhow!("Time went backwards"))
}

