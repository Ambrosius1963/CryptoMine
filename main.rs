use sha2::{Sha256, Digest};
use reqwest::blocking::Client;
use serde_json::Value;
use std::sync::{Arc, AtomicBool, Ordering};
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

/// Multi-threaded mining function
fn multi_threaded_mining(prev_hash: String, merkle_root: String, target_difficulty: &str, num_threads: usize) {
    //Converts the hex difficulty target into a numeric value for comparison
    let difficulty_num = u128::from_str_radix(target_difficulty, 16).unwrap_or(u128::MAX);
    //A thread-safe flag that lets all threads check if mining is finished
    let mining_complete = Arc::new(AtomicBool::new(false));

    let start_time = Instant::now();
    let mut handles = vec![];

    for thread_id in 0..num_threads {
        let prev_hash = prev_hash.clone();
        let merkle_root = merkle_root.clone();
        let mining_complete = Arc::clone(&mining_complete);

        let handle = thread::spawn(move || {                    // Each thread mines its own range of nonces
            let mut nonce = thread_id as u64 * 1_000_000_000;

            loop {
                if mining_complete.load(Ordering::Relaxed) {
                    break;
                }

                // Hash the Data
                let input = format!("{}{}{}", prev_hash, merkle_root, nonce);
                let mut hasher = Sha256::new();
                hasher.update(input.as_bytes());
                let hash_result = hasher.finalize();
                let hash_hex = hex::encode(hash_result);

                // Convert Hash to a Number and Compare to Difficulty
                let hash_num = u128::from_str_radix(&hash_hex[..32], 16).unwrap_or(u128::MAX);

                //  If a Valid Block is Found
                if hash_num <= difficulty_num {
                    let duration = start_time.elapsed();
                    println!("✅ Block Mined! Nonce: {}", nonce);
                    println!("🔗 Hash: {}", hash_hex);
                    println!("⏱️ Time Taken: {:.2?}", duration);

                    submit_block(&hash_hex);
                    mining_complete.store(true, Ordering::Relaxed);
                    break;
                }
                
                // Increase Nonce and Print Progress
                nonce += 1;

                if nonce % 1_000_000 == 0 {
                    println!("Thread {}: Attempts: {} | Last hash: {}", thread_id, nonce, hash_hex);
                }
            }
        });

        handles.push(handle);
    }

    // Wait for All Threads to Finish
    for handle in handles {
        handle.join().unwrap();
    }
}

/// Handles WebSocket connection with auto-reconnect logic
async fn connect_websocket() {
    loop {
        let url = Url::parse("wss://ws.blockchain.info/inv").unwrap();
        println!("\n🔌 Connecting to Blockchain WebSocket API...");

        // Calls Once Connection has been made
        match connect_async(url).await {
            Ok((mut ws_stream, _)) => {
                println!("\n✅ Connected to WebSocket!\n");

                //Subscribe to new block notifications
                let subscribe_msg = json!({ "op": "ping_block" }).to_string();
                if let Err(err) = ws_stream.send(Message::Text(subscribe_msg)).await {
                    eprintln!("❌ Subscription error: {:?}", err);
                    continue;
                }
                println!("📡 Subscribed to new Bitcoin blocks...");
                
                //Waits for a message, turning the received message into text
                while let Some(msg) = ws_stream.next().await {
                    if let Ok(msg) = msg {
                        if let Ok(text) = msg.to_text() {
                            if let Ok(parsed) = serde_json::from_str::<Value>(text) {

                                // Extract Block Data
                                if let Some(op) = parsed.get("op").and_then(|o| o.as_str()) {
                                    if op == "block" {
                                        if let Some(block) = parsed.get("x") {
                                            let prev_hash = block.get("hash").and_then(|h| h.as_str()).unwrap_or("UNKNOWN").to_string();
                                            let merkle_root = block.get("mrklRoot").and_then(|m| m.as_str()).unwrap_or("UNKNOWN").to_string();
                                            let difficulty = "000000000000000000000000000000000000"; // Simulated difficulty
                                            
                                            // New Block Found
                                            println!("🔹 New Block Found!");
                                            println!("📌 Prev Hash: {}", prev_hash);
                                            println!("🌿 Merkle Root: {}", merkle_root);

                                            // Begin mining with 8 threads
                                            multi_threaded_mining(prev_hash, merkle_root, &difficulty, 8);
                                        }
                                    }
                                }
                            } else {
                                eprintln!("❌ Failed to parse WebSocket message!");
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("🔴 WebSocket connection failed: {:?}", e);
            }
        }

        println!("🔄 Reconnecting in 5 seconds...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

/// Entry point
#[tokio::main]
async fn main() {
    println!("Starting WebSocket connection...");
    connect_websocket().await;
}