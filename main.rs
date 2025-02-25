use num_bigint::BigUint;
use std::time::Duration;
use num_traits::{ToPrimitive, FromPrimitive};
use sha2::{Sha256, Digest};
use reqwest::blocking::Client;
use serde_json::Value;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::io::{self, Write};
use std::thread;
use std::time::Instant;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};
use serde_json::json;
use url::Url;

/// Retrieves the latest block hash from a Bitcoin Core node
fn get_latest_block_hash() -> Option<String> {
    let client = Client::new();
    let url = "http://127.0.0.1:8332"; // Bitcoin Core RPC URL

    let body = json!({
        "jsonrpc": "1.0",
        "id": "getblocktemplate",
        "method": "getblocktemplate",
        "params": [{"rules": ["segwit"]}]
    });

    if let Ok(response) = client.post(url)
        .header("Content-Type", "application/json")
        .basic_auth("your_rpc_user", Some("your_rpc_password"))
        .json(&body)
        .send()
    {
        if let Ok(json) = response.json::<Value>() {
            return json.get("result")
                .and_then(|r| r.get("previousblockhash"))
                .and_then(|h| h.as_str())
                .map(|s| s.to_string());
        }
    }

    None
}

/// Submits the mined block to Bitcoin Core
fn submit_block(block_data: &str) {
    let client = Client::new();
    let url = "http://127.0.0.1:8332";

    let body = json!({
        "jsonrpc": "1.0",
        "id": "submitblock",
        "method": "submitblock",
        "params": [block_data]
    });

    match client.post(url)
        .header("Content-Type", "application/json")
        .basic_auth("your_rpc_user", Some("your_rpc_password"))
        .json(&body)
        .send()
    {
        Ok(res) => println!("✔️ Block submission response: {:?}", res.text().unwrap_or_default()),
        Err(e) => eprintln!("❌ Error submitting block: {:?}", e),
    }
}

fn calculate_target(difficulty: f64) -> BigUint {
    let max_target = BigUint::parse_bytes(
        b"00000000FFFF0000000000000000000000000000000000000000000000000000",
        16
    ).unwrap();

    let difficulty_bn = BigUint::from_f64(difficulty).unwrap_or(BigUint::from(1_u32));

    &max_target / difficulty_bn
}

fn clear_console() {
    print!("\x1B[2J\x1B[H"); // Clears the screen and moves the cursor to the home position (top-left corner)
    io::stdout().flush().unwrap(); // Ensures the command is immediately flushed to stdout
}

/// Multi-threaded mining function
fn multi_threaded_mining(prev_hash: String, merkle_root: String, target_difficulty: &str, num_threads: usize) { 
    let difficulty_num = calculate_target(target_difficulty.parse::<f64>().unwrap_or(1.0));
    let mining_complete = Arc::new(AtomicBool::new(false));
    let start_time = Instant::now();
    let mut handles = vec![];
    let mut attempted = 0;
    let mut write = 0;

    for thread_id in 0..num_threads {
        let prev_hash = prev_hash.clone();
        let merkle_root = merkle_root.clone();
        let mining_complete = Arc::clone(&mining_complete);
        let difficulty_num_clone = difficulty_num.clone();

        let handle = thread::spawn(move || {
            let mut nonce = thread_id as u64 * 500_000_000;

            loop {
                if mining_complete.load(Ordering::Relaxed) {
                    break;
                }

                let input = format!("{}{}{}", prev_hash, merkle_root, nonce);
                let mut hasher = Sha256::new();
                hasher.update(input.as_bytes());
                let hash_result = hasher.finalize();
                let hash_hex = hex::encode(hash_result);

                if let Ok(hash_num) = u128::from_str_radix(&hash_hex[..32], 16) {
                    if BigUint::from(hash_num) <= difficulty_num_clone { // We won!
                        let duration = start_time.elapsed();
                        println!("✅ Block Mined! Nonce: {}", nonce);
                        println!("🔗 Hash: {}", hash_hex);
                        println!("⏱️ Time Taken: {:.2?}", duration);

                        submit_block(&hash_hex);
                        mining_complete.store(true, Ordering::Relaxed);
                        break;
                    }
                }

                nonce += 1;
                attempted += 1;

                if nonce % 1_000_000 == 0 {
                    write += 1;
                    if write == 2 && thread_id == 0 {
                        clear_console();
                        write = 0;
                    }
                    println!("Thread {} Report: Attempted Nonces: {}-{}, Last attempted hex: {}", thread_id, 500_000_000 * thread_id, nonce, hash_hex);
                }

                if attempted % 1357 == 0 && thread_id == 0 {
                    println!("Total Attempted Nonces: {}", attempted * 8);
                    print!("\x1B[F");
                }
            }
            
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

// Handles WebSocket connection with auto-reconnect logic
async fn connect_websocket() {
    loop {
        let url = Url::parse("wss://ws.blockchain.info/inv").unwrap();
        println!("\n🔌 Connecting to Blockchain WebSocket API...");

        match connect_async(url).await {
            Ok((mut ws_stream, _)) => {
                println!("\n✅ Connected to WebSocket!\n");

                let subscribe_msg = json!({ "op": "blocks_sub" }).to_string();
                if let Err(err) = ws_stream.send(Message::Text(subscribe_msg)).await {
                    eprintln!("❌ Subscription error: {:?}", err);
                    continue;
                }
                println!("📡 Subscribed to new Bitcoin blocks...");
                
                //while let Some(msg) = ws_stream.next().await {
                //    if let Ok(msg) = msg {
                //        if let Ok(text) = msg.to_text() {
                //            if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                //                if let Some(op) = parsed.get("op").and_then(|o| o.as_str()) {
                //                    if op == "block" {
                //                        if let Some(block) = parsed.get("x") {
                //                            let prev_hash = block.get("hash").and_then(|h| h.as_str()).unwrap_or("UNKNOWN").to_string();
                //                            let merkle_root = block.get("mrklRoot").and_then(|m| m.as_str()).unwrap_or("UNKNOWN").to_string();
                //                            let difficulty = "000000000000000000000000000000000000"; // Simulated difficulty
                //                            
                //                            println!("🔹 New Block Found!");
                //                            println!("📌 Prev Hash: {}", prev_hash);
                //                            println!("🌿 Merkle Root: {}", merkle_root);
//
                //                            multi_threaded_mining(prev_hash, merkle_root, &difficulty, 8);
                //                        }
                //                    }
                //                }
                //            } else {
                //                eprintln!("❌ Failed to parse WebSocket message!");
                //            }
                //        }
                //    }
                //}
                let prev_hash = "00000000000000000000a9b103f6ec3588f19b27e10101f255b845b042a61d8d".to_string();
                let merkle_root = "c10c12f6d7e2a75ad2864692a6912890a3358b1bb200775f441b40203fd273cb".to_string();
                let difficulty = "9999999999999999999999999999999999999999999999999110.69"; // difficulty
                
                println!("🔹 New Block Found!");
                println!("📌 Prev Hash: {}", prev_hash);
                println!("🌿 Merkle Root: {}", merkle_root);

                multi_threaded_mining(prev_hash, merkle_root, &difficulty, 8);
            }
            Err(e) => {
                eprintln!("🔴 WebSocket connection failed: {:?}", e);
            }
        }

        println!("🔄 Reconnecting in 5 seconds...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

// Entry point
#[tokio::main]
async fn main() {
    println!("Starting WebSocket connection...");
    connect_websocket().await;
}
