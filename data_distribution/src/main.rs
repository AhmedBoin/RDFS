use rand::Rng;
use rand::seq::SliceRandom;
use raptorq::{Decoder, Encoder, EncodingPacket};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::time::Instant;

fn main() {
    // Create raptor_data directory
    let folder_path = "data_distribution/raptor_data";
    let _ = fs::remove_dir_all(folder_path); // clean before start
    fs::create_dir_all(folder_path).expect("Failed to create folder");

    // Generate some random data to send
    let mut data: Vec<u8> = vec![0; 1_000_000_000]; // 1.28 MB
    println!("filling with random data");
    let start = Instant::now();
    rand::rng().fill(&mut data[..]);
    println!("Finished filling with random data in {:?}", start.elapsed());

    // Create the Encoder, with an MTU of 1400 (common for Ethernet)
    println!("Encoding data...");
    let start = Instant::now();
    let encoder = Encoder::with_defaults(&data, 1_280);
    println!("Finished Encoding data in {:?}", start.elapsed());

    // Encode and save packets to files
    let start = Instant::now();
    let packets = encoder
        .get_encoded_packets(0)
        .iter()
        .map(|packet| packet.serialize())
        .collect::<Vec<Vec<u8>>>();
    println!(
        "Data generated: {} in {:?}",
        packets.len() * packets[0].len(),
        start.elapsed()
    );
    return;

    for (i, packet) in packets.iter().enumerate() {
        let filename = format!("{}/packet_{:02}.bin", folder_path, i);
        let mut file = File::create(&filename).expect("Failed to write packet");
        file.write_all(packet).expect("Failed to write packet data");
    }

    // Simulate packet loss by deleting 10 random packet files
    let mut files: Vec<_> = fs::read_dir(folder_path)
        .expect("Failed to read directory")
        .map(|res| res.expect("Failed to read entry").path())
        .collect();

    files.shuffle(&mut rand::rng());
    for f in files.iter().take(10) {
        fs::remove_file(f).expect("Failed to delete file");
    }

    // Prepare decoder using the encoder config
    let mut decoder = Decoder::new(encoder.get_config());

    // Load remaining packets and decode
    let mut remaining_files: Vec<_> = fs::read_dir(folder_path)
        .expect("Failed to read directory")
        .map(|res| res.expect("Failed to read entry").path())
        .collect();

    let mut result = None;
    while let Some(packet_path) = remaining_files.pop() {
        let mut file = File::open(packet_path).expect("Failed to open packet file");
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).expect("Failed to read file");

        result = decoder.decode(EncodingPacket::deserialize(&buffer));
        if result.is_some() {
            break;
        }
    }

    // Confirm successful reconstruction
    match result {
        Some(reconstructed) => {
            assert_eq!(reconstructed, data);
            println!("Success: Data reconstructed!");
        }
        None => {
            eprintln!("Error: Failed to reconstruct data.");
        }
    }
}
