// SPDX-License-Identifier: Apache-2.0

use soroban_sdk::{contracttype, Address, BytesN, String};

#[contracttype]
pub enum TokenDataKey {
    Balance(Address),
    Allowance(Address, Address),
    TotalSupply,
    Admin,
    PendingAdmin,
    AdminNonce,
    Paused,
    MaxSupply,
    Metadata,
    PendingUpgrade,
    Version,
    /// TOK-33: optional admin-configured per-recipient cap for `mint_to_batch`.
    RecipientCap,
}

/// SEP-41 token metadata (TOK-32): name, symbol and decimals in one struct.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenMetadata {
    pub decimal: u32,
    pub name: String,
    pub symbol: String,
}

/// TOK-18/TOK-10: a scheduled contract WASM upgrade, pending its time-lock.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingUpgrade {
    pub wasm_hash: BytesN<32>,
    pub unlock_time: u64,
}
