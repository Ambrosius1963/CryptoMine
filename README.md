# CryptoMine Overview
CryptoMine is a project aimed at deepening my understanding of blockchain technology, cryptocurrency mining, and software engineering best practices. This project allows me to explore real-time WebSocket communication, cryptographic hashing (SHA-256), and parallel computing while implementing a basic Bitcoin mining algorithm.

## Description
This CryptoMine is a Rust-based Bitcoin mining application that connects to the Blockchain.com WebSocket API to receive real-time block data. It extracts the necessary components, such as the previous block hash and Merkle root, then performs mining by iterating through nonce values to find a valid SHA-256 hash that meets the required difficulty target.

## Purpose
The purpose of this project is to:
- Gain hands-on experience with cryptocurrency mining algorithms.
- Learn how to interact with real-time blockchain data via WebSockets.
- Explore cryptographic hashing and Proof of Work consensus mechanisms.
- Improve my Rust programming skills and apply concurrency for performance optimization.

# Development Environment

## Tools
- **Presentation Scipt** - [Script link](https://docs.google.com/document/d/1IB-TiJC02yjrdiHLuEEWTivqUQY1u4DY0oFXVIu_Y4M/edit?usp=sharing)
- **Live Share** - Used to work together with live updates.
- **Blockchain.com WebSocket API** – Provides real-time Bitcoin transaction and block data.
- **Rust** – The main programming language for the application using VS Code.
- **Tokio** – An asynchronous runtime for handling WebSocket connections.
- **Tokio-Tungstenite** – A WebSocket client for Rust.
- **SHA2 crate** – Used to compute SHA-256 hashes.
- **Serde & Serde-JSON** – Used for parsing JSON data from the API.
- **Hex crate** – For encoding/decoding hexadecimal data.

## Language
CryptoMine is developed using **Rust** for its safety, performance, and concurrency capabilities.

# Useful Websites
* [Blockchain.com API](https://www.blockchain.com/api) – Provides real-time Bitcoin network data.
* [Rust Programming Language](https://www.rust-lang.org/) – Official Rust documentation and resources.
* [Tokio Async Runtime](https://tokio.rs/) – Documentation on asynchronous programming in Rust.
* [Rust Crates](https://crates.io/) – A registry of Rust libraries, including those used in this project.
* [Bitcoin Whitepaper](https://bitcoin.org/bitcoin.pdf) – The original paper by Satoshi Nakamoto explaining Bitcoin and Proof of Work.
