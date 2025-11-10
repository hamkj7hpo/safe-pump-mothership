//! Mothership v4.0 — WASM CDN + FULL DASHBOARD
//! BLS12-381 + Raydium CPMM + Burn Laser + Badge + Vault + Payout
//! NO SPL, NO ANCHOR — All on-chain logic in warp_core + raydium-cp-swap

use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use wasm_bindgen::prelude::*;
use std::str::FromStr;

// ─────────────────────────────────────────────────────────────────────────────
// HARD CODED PROGRAM IDS
// ─────────────────────────────────────────────────────────────────────────────
pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKLVAQJ4uM3n8vQJ4uM3n8vQ");
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const RAYDIUM_CP_SWAP_PROGRAM_ID: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
pub const SYSTEM_PROGRAM_ID: Pubkey = pubkey!("11111111111111111111111111111111");
pub const MOTHERSHIP_PROGRAM_ID: Pubkey = pubkey!("AymD4HzxTN2SK6UDrCcXD2uAFk4RptvQKzMT5P9GSr32");

// ─────────────────────────────────────────────────────────────────────────────
// CONSTANTS (MATCH ON-CHAIN)
// ─────────────────────────────────────────────────────────────────────────────
pub const GLOBAL_TAX_BPS: u64 = 100;
pub const GLOBAL_LP_TAX_BPS: u64 = 50;
pub const SWAPPER_REWARD_TAX_BPS: u64 = 40;
pub const BADGE_REWARD_TAX_BPS: u64 = 10;

pub const MAX_AIRDROP_CLAIMERS: usize = 10_000;
pub const AIRDROP_TRIGGER_COUNT: usize = 1000;
pub const AIRDROP_PER_USER_BPS: u64 = 1;

pub const MAX_BADGE_HOLDERS: usize = 1000;
pub const BUY_SWAPS_FOR_BADGE: u64 = 1000;
pub const REWARD_DISTRIBUTION_PERIOD: i64 = 86_400;

pub const ANTI_SNIPER_COOLDOWN: i64 = 120;
pub const SELL_COOLDOWN: i64 = 86_400;

pub const MAX_SUPPLY: u64 = 1_000_000_000_000_000_000;
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

pub const FIB_START_BPS: u64 = 1;
pub const FIB_TIERS: [u64; 8] = [1, 2, 3, 5, 8, 13, 21, 34];
pub const FIB_MCAP_THRESHOLDS_SOL: [u64; 8] = [
    100_000, 500_000, 1_000_000, 5_000_000,
    10_000_000, 25_000_000, 50_000_000, 75_000_000,
];
pub const MAX_SWAP_BPS_AT_100M: u64 = 100;

// ─────────────────────────────────────────────────────────────────────────────
// ATA & POOL PDA
// ─────────────────────────────────────────────────────────────────────────────
pub fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[wallet.as_ref(), TOKEN_PROGRAM_ID.as_ref(), mint.as_ref()],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    ).0
}

pub fn get_raydium_pool_pda(mint_a: &Pubkey, mint_b: &Pubkey) -> Pubkey {
    let (pda, _) = Pubkey::find_program_address(
        &[
            b"pool",
            mint_a.as_ref(),
            mint_b.as_ref(),
            RAYDIUM_CP_SWAP_PROGRAM_ID.as_ref(),
        ],
        &RAYDIUM_CP_SWAP_PROGRAM_ID,
    );
    pda
}

// ─────────────────────────────────────────────────────────────────────────────
// MOTHERSHIP CLIENT
// ─────────────────────────────────────────────────────────────────────────────
#[wasm_bindgen]
pub struct MothershipClient {
    bls_pk: Vec<u8>,
    solana_kp: Keypair,
}

#[wasm_bindgen]
impl MothershipClient {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let bls_pk = vec![0u8; 48];
        let solana_kp = Keypair::new();
        MothershipClient { bls_pk, solana_kp }
    }

    #[wasm_bindgen]
    pub fn get_bls_pk(&self) -> Vec<u8> { self.bls_pk.clone() }

    #[wasm_bindgen]
    pub fn get_solana_pk(&self) -> String { self.solana_kp.pubkey().to_string() }

    #[wasm_bindgen]
    pub fn test_math(&self) -> u32 { 2 + 2 }
}

// ─────────────────────────────────────────────────────────────────────────────
// DASHBOARD: BURN LASER
// ─────────────────────────────────────────────────────────────────────────────
#[wasm_bindgen]
pub struct BurnLaserEvent {
    pub token_mint: String,
    pub percent: u8,
    pub amount_burned: u64,
    pub timestamp: i64,
}

#[wasm_bindgen]
impl BurnLaserEvent {
    #[wasm_bindgen(constructor)]
    pub fn new(token_mint: &str, percent: u8, amount_burned: u64, timestamp: i64) -> Self {
        BurnLaserEvent {
            token_mint: token_mint.to_string(),
            percent,
            amount_burned,
            timestamp,
        }
    }
}

static mut LAST_BURN: Option<BurnLaserEvent> = None;

#[wasm_bindgen]
pub fn record_burn_laser(token_mint: &str, percent: u8, amount_burned: u64, timestamp: i64) {
    unsafe {
        LAST_BURN = Some(BurnLaserEvent::new(token_mint, percent, amount_burned, timestamp));
    }
}

#[wasm_bindgen]
pub fn get_last_burn() -> Option<JsValue> {
    unsafe {
        LAST_BURN.as_ref().map(|e| {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"token_mint".into(), &e.token_mint.clone().into()).unwrap();
            js_sys::Reflect::set(&obj, &"percent".into(), &(e.percent as f64).into()).unwrap();
            js_sys::Reflect::set(&obj, &"amount_burned".into(), &(e.amount_burned as f64).into()).unwrap();
            js_sys::Reflect::set(&obj, &"timestamp".into(), &(e.timestamp as f64).into()).unwrap();
            JsValue::from(obj)
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DASHBOARD: BADGE HOLDERS
// ─────────────────────────────────────────────────────────────────────────────
#[wasm_bindgen]
pub fn get_badge_holders_pda(mint: &str) -> String {
    let mint_pk = Pubkey::from_str(mint).unwrap_or_default();
    let (pda, _) = Pubkey::find_program_address(
        &[b"badge-holders", mint_pk.as_ref()],
        &MOTHERSHIP_PROGRAM_ID,
    );
    pda.to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// DASHBOARD: VAULT BALANCE & NEXT PAYOUT
// ─────────────────────────────────────────────────────────────────────────────
#[wasm_bindgen]
pub fn get_sol_vault_pda(owner: &str) -> String {
    let owner_pk = Pubkey::from_str(owner).unwrap_or_default();
    let (pda, _) = Pubkey::find_program_address(
        &[b"contract", owner_pk.as_ref()],
        &MOTHERSHIP_PROGRAM_ID,
    );
    get_associated_token_address(&pda, &Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap()).to_string()
}

#[wasm_bindgen]
pub fn get_next_payout_time(last_dist: i64) -> i64 {
    last_dist + REWARD_DISTRIBUTION_PERIOD
}

#[wasm_bindgen]
pub fn format_sol(lamports: u64) -> f64 {
    lamports as f64 / LAMPORTS_PER_SOL as f64
}

// ─────────────────────────────────────────────────────────────────────────────
// TAX & FIBONACCI
// ─────────────────────────────────────────────────────────────────────────────
#[wasm_bindgen]
pub fn calculate_swap_tax(amount_in: u64) -> JsValue {
    let total_tax = amount_in * GLOBAL_TAX_BPS / 10_000;
    let lp_tax = total_tax * GLOBAL_LP_TAX_BPS / GLOBAL_TAX_BPS;
    let swapper_tax = total_tax * SWAPPER_REWARD_TAX_BPS / GLOBAL_TAX_BPS;
    let badge_tax = total_tax * BADGE_REWARD_TAX_BPS / GLOBAL_TAX_BPS;
    let net_amount = amount_in - total_tax;

    let result = js_sys::Object::new();
    js_sys::Reflect::set(&result, &"total_tax".into(), &(total_tax as f64).into()).unwrap();
    js_sys::Reflect::set(&result, &"lp_tax".into(), &(lp_tax as f64).into()).unwrap();
    js_sys::Reflect::set(&result, &"swapper_tax".into(), &(swapper_tax as f64).into()).unwrap();
    js_sys::Reflect::set(&result, &"badge_tax".into(), &(badge_tax as f64).into()).unwrap();
    js_sys::Reflect::set(&result, &"net_amount".into(), &(net_amount as f64).into()).unwrap();
    JsValue::from(result)
}

#[wasm_bindgen]
pub fn get_fib_buy_cap(mcap_lamports: u64, total_supply: u64) -> u64 {
    let mcap_sol = mcap_lamports / LAMPORTS_PER_SOL;
    let tier = FIB_TIERS.iter().enumerate()
        .find(|(i, _)| mcap_sol >= FIB_MCAP_THRESHOLDS_SOL[*i])
        .map(|(i, _)| i as u8)
        .unwrap_or(7);
    let current_bps = if mcap_sol >= 100_000_000 {
        MAX_SWAP_BPS_AT_100M
    } else {
        FIB_START_BPS + FIB_TIERS[tier as usize]
    };
    total_supply * current_bps / 10_000
}

// ─────────────────────────────────────────────────────────────────────────────
// EXPORT TO JS
// ─────────────────────────────────────────────────────────────────────────────
#[wasm_bindgen]
pub fn get_associated_token_address_js(wallet: &str, mint: &str) -> String {
    let wallet_pk = Pubkey::from_str(wallet).unwrap_or_default();
    let mint_pk = Pubkey::from_str(mint).unwrap_or_default();
    get_associated_token_address(&wallet_pk, &mint_pk).to_string()
}

#[wasm_bindgen]
pub fn get_raydium_pool_pda_js(mint_a: &str, mint_b: &str) -> String {
    let mint_a_pk = Pubkey::from_str(mint_a).unwrap_or_default();
    let mint_b_pk = Pubkey::from_str(mint_b).unwrap_or_default();
    get_raydium_pool_pda(&mint_a_pk, &mint_b_pk).to_string()
}
