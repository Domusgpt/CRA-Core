//! Basic CSPM transmission example.
//!
//! Demonstrates encoding, simulated noisy channel, and decoding.

use cspm_core::{
    CspmEncoder, CspmDecoder, GenesisConfig,
    quaternion::Quaternion,
    trace_integration::CspmTraceEmitter,
};

fn main() {
    println!("=== CSPM Basic Transmission Demo ===\n");

    // 1. Initialize with shared secret
    let shared_secret = b"my_secure_shared_secret_2025";
    let config = GenesisConfig::new(shared_secret);
    let genesis_hash = config.genesis_hash; // Save for later use

    println!("Genesis Hash: {}", hex::encode(&genesis_hash[..8]));
    println!();

    // 2. Create encoder and decoder
    let mut encoder = CspmEncoder::new(config.clone());
    let mut decoder = CspmDecoder::new(config);

    // 3. Create TRACE emitter for logging
    let mut trace = CspmTraceEmitter::new();

    // 4. Encode some data
    let message = b"Hello, CSPM!";
    println!("Original message: {:?}", String::from_utf8_lossy(message));
    println!();

    let symbols = encoder.encode_bytes(message).expect("Encoding failed");
    println!("Encoded {} bytes into {} symbols", message.len(), symbols.len());
    println!();

    // 5. Simulate noisy channel
    println!("Simulating noisy channel...");
    let noise_level = 0.02; // 2% noise

    let noisy_symbols: Vec<Quaternion> = symbols
        .iter()
        .map(|s| {
            // Add Gaussian noise
            let noise_w = (rand::random::<f64>() - 0.5) * noise_level;
            let noise_x = (rand::random::<f64>() - 0.5) * noise_level;
            let noise_y = (rand::random::<f64>() - 0.5) * noise_level;
            let noise_z = (rand::random::<f64>() - 0.5) * noise_level;

            Quaternion::new(
                s.quaternion.w + noise_w,
                s.quaternion.x + noise_x,
                s.quaternion.y + noise_y,
                s.quaternion.z + noise_z,
            ).normalize()
        })
        .collect();

    // 6. Decode
    println!("\nDecoding with geometric error correction...");
    let decoded_bytes = decoder.decode_to_bytes(&noisy_symbols).expect("Decoding failed");

    // 7. Show results
    println!("\nDecoded message: {:?}", String::from_utf8_lossy(&decoded_bytes));
    println!();

    // 8. Show statistics
    let stats = decoder.stats();
    println!("=== Decoding Statistics ===");
    println!("Total symbols:      {}", stats.total_symbols);
    println!("Corrected symbols:  {}", stats.corrected_symbols);
    println!("Error rate:         {:.2}%", decoder.error_rate() * 100.0);
    println!("Avg correction:     {:.4}", stats.avg_correction_distance);
    println!("Max correction:     {:.4}", stats.max_correction_distance);
    println!();

    // 9. Verify match
    let matches = decoded_bytes.iter()
        .zip(message.iter())
        .all(|(a, b)| a == b);

    if matches {
        println!("SUCCESS: Message decoded correctly with {} corrections!",
                 stats.corrected_symbols);
    } else {
        println!("MISMATCH: Some bytes differ (this should be rare)");
    }

    // 10. Show some TRACE events
    println!("\n=== Sample TRACE Events ===");

    // Emit some events for demonstration
    trace.emit_link_init(&cspm_core::crypto::ChainState::genesis(&genesis_hash));

    for symbol in symbols.iter().take(3) {
        trace.emit_packet_tx(
            &cspm_core::crypto::ChainState::genesis(&genesis_hash),
            &symbol.optical,
            symbol.vertex_index,
            symbol.bits,
            1,
        );
    }

    for event in trace.events().iter().take(4) {
        println!("\n{}", serde_json::to_string_pretty(event).unwrap());
    }
}
