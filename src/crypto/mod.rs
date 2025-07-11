// ABOUTME: Cryptography module providing secure encryption and key management
// ABOUTME: Centralizes all cryptographic operations for the pierre_mcp_server
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Cryptographic utilities for Pierre MCP Server

pub mod keys;

pub use keys::{A2AKeyManager, A2AKeypair, A2APublicKey};
