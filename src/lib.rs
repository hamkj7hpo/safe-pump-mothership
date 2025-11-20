//! MOTHERSHIP v4.1 — ZK Rotator + SPMP Handshake + Vanity PDA
//! Program ID: JBjKCmvSK3dMPfKk1WGD8nZfw8yAZHtuZ3GLo7NpCHX7
//! NO SPL, NO ANCHOR — Pure WASM + warp_core

use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use wasm_bindgen::prelude::*;
use std::str::FromStr;
use blstrs::{G1Projective, G2Projective};
use getrandom::getrandom::fill;
use borsh::{BorshSerialize, BorshDeserialize};
use serde::Serialize;

// ─────────────────────────────────────────────────────────────────────────────
// PROGRAM ID & CONSTANTS
// ─────────────────────────────────────────────────────────────────────────────
pub const MOTHERSHIP_PROGRAM_ID: Pubkey = pubkey!("JBjKCmvSK3dMPfKk1WGD8nZfw8yAZHtuZ3GLo7NpCHX7");
pub const SPMP_SUFFIX: &str = "SPMP";
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

// Global Tax: 2.5%
pub const GLOBAL_TAX_BPS: u64 = 250;

// Fibonacci Velocity (SOL per block)
pub const FIB_VELOCITY_SOL: [u64; 8] = [1, 3, 7, 15, 30, 70, 150, 300];
pub const FIB_MCAP_THRESHOLDS_SOL: [u64; 8] = [
    1_000_000, 3_000_000, 7_000_000, 15_000_000,
    30_000_000, 70_000_000, 150_000_000, 300_000_000,
];
pub const FIB_START_BPS: u64 = 1;
pub const MAX_SWAP_BPS_AT_TOP: u64 = 100;
pub const FIB_TIERS: [u64; 8] = [1, 2, 3, 5, 8, 13, 21, 34];

// ─────────────────────────────────────────────────────────────────────────────
// PDA HELPERS
// ─────────────────────────────────────────────────────────────────────────────
pub fn get_vault_pda(user: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"zk_vault", user.as_ref()], &MOTHERSHIP_PROGRAM_ID).0
}

pub fn get_meme_pda(name: &[u8], spmp_mint: &[u8], bump: u8) -> Pubkey {
    Pubkey::find_program_address(&[b"meme", name, spmp_mint, &[bump]], &MOTHERSHIP_PROGRAM_ID).0
}

pub fn get_block_swap_state_pda(slot: u64) -> Pubkey {
    Pubkey::find_program_address(&[b"swap-state", &slot.to_le_bytes()], &MOTHERSHIP_PROGRAM_ID).0
}

// ─────────────────────────────────────────────────────────────────────────────
// STATE
// ─────────────────────────────────────────────────────────────────────────────
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MemeRegistry {
    pub vanity_program_id: Pubkey,
    pub spmp_mint: String,
    pub deployer: Pubkey,
    pub created_at: i64,
    pub is_active: bool,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct HandshakeResponse {
    pub vanity_program_id: Pubkey,
    pub spmp_mint: String,
    pub rotator_pk: Pubkey,
    pub mothership_pda: Pubkey,
}

// ─────────────────────────────────────────────────────────────────────────────
// MOTHERSHIP CLIENT
// ─────────────────────────────────────────────────────────────────────────────
#[wasm_bindgen]
pub struct MothershipClient {
    bls_sk: [u8; 32],
    bls_pk: [u8; 48],
    deployer_kp: Keypair,
    registry: Vec<MemeRegistry>,
}

#[wasm_bindgen]
impl MothershipClient {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let mut bls_sk = [0u8; 32];
        fill(&mut bls_sk).unwrap();
        let bls_pk = (G1Projective::generator() * blstrs::Scalar::from_bytes(&bls_sk).unwrap()).to_compressed();
        let deployer_kp = Keypair::new();

        MothershipClient {
            bls_sk,
            bls_pk,
            deployer_kp,
            registry: Vec::new(),
        }
    }

    #[wasm_bindgen]
    pub fn get_deployer(&self) -> String {
        self.deployer_kp.pubkey().to_string()
    }

    #[wasm_bindgen]
    pub fn get_bls_pk(&self) -> Vec<u8> {
        self.bls_pk.to_vec()
    }

    // ───── HANDSHAKE: Register Meme + Generate Vanity ID ─────
    #[wasm_bindgen]
    pub fn register_meme(&mut self, name: &str, symbol: &str) -> JsValue {
        let deployer = self.deployer_kp.pubkey();
        let spmp_mint = format!("{}{}", symbol.to_uppercase(), SPMP_SUFFIX);

        // 1. Derive mothership PDA
        let (mothership_pda, _) = Pubkey::find_program_address(
            &[b"contract", deployer.as_ref()],
            &MOTHERSHIP_PROGRAM_ID,
        );

        // 2. Mine vanity program ID with SPMP suffix
        let mut vanity_id = Pubkey::default();
        let mut bump: u8 = 0;
        let mut nonce: u8 = 0;

        loop {
            let (pda, b) = Pubkey::find_program_address(
                &[b"meme", name.as_bytes(), spmp_mint.as_bytes(), &[nonce]],
                &MOTHERSHIP_PROGRAM_ID,
            );
            if pda.to_string().ends_with(SPMP_SUFFIX) {
                vanity_id = pda;
                bump = b;
                break;
            }
            nonce = nonce.wrapping_add(1);
            if nonce == 0 { break; }
        }

        // 3. Generate rotator
        let mut rotator_sk = [0u8; 32];
        fill(&mut rotator_sk).unwrap();
        let rotator_kp = Keypair::from_bytes(&[&rotator_sk, &[0; 32]].concat()).unwrap();
        let rotator_pk = rotator_kp.pubkey();

        // 4. Register
        let registry_entry = MemeRegistry {
            vanity_program_id: vanity_id,
            spmp_mint: spmp_mint.clone(),
            deployer,
            created_at: js_sys::Date::now() as i64 / 1000,
            is_active: true,
        };
        self.registry.push(registry_entry.clone());

        // 5. Return handshake response
        let resp = HandshakeResponse {
            vanity_program_id: vanity_id,
            spmp_mint,
            rotator_pk,
            mothership_pda,
        };

        JsValue::from_serde(&resp).unwrap()
    }

    // ───── ROTATOR: Hourly Key Rotation ─────
    #[wasm_bindgen]
    pub fn rotate_rotator(&mut self, vanity_id: &str) -> Option<String> {
        let vanity_pk = Pubkey::from_str(vanity_id).ok()?;
        let entry = self.registry.iter_mut().find(|e| e.vanity_program_id == vanity_pk)?;
        if !entry.is_active { return None; }

        let now = js_sys::Date::now() as i64 / 1000;
        if now - entry.created_at < 3600 { return None; }

        let mut sk = [0u8; 32];
        fill(&mut sk).unwrap();
        let kp = Keypair::from_bytes(&[&sk, &[0; 32]].concat()).unwrap();
        Some(kp.pubkey().to_string())
    }

    // ───── FIB VELOCITY & CAP ─────
    #[wasm_bindgen]
    pub fn get_velocity_limit(&self, mcap_sol: u64) -> u64 {
        let tier = FIB_MCAP_THRESHOLDS_SOL.iter()
            .position(|&t| mcap_sol >= t)
            .unwrap_or(7);
        FIB_VELOCITY_SOL[tier] * LAMPORTS_PER_SOL
    }

    #[wasm_bindgen]
    pub fn get_buy_cap(&self, mcap_lamports: u64, supply: u64, top_tier_sol: u64) -> u64 {
        let mcap_sol = mcap_lamports / LAMPORTS_PER_SOL;
        let tier = FIB_MCAP_THRESHOLDS_SOL.iter()
            .position(|&t| mcap_sol >= t)
            .unwrap_or(7);
        let bps = if mcap_sol >= top_tier_sol {
            MAX_SWAP_BPS_AT_TOP
        } else {
            FIB_START_BPS + FIB_TIERS[tier]
        };
        supply * bps / 10_000
    }

    // ───── TAX ─────
    #[wasm_bindgen]
    pub fn calculate_tax(&self, amount_in: u64) -> JsValue {
        let tax = amount_in * GLOBAL_TAX_BPS / 10_000;
        let net = amount_in - tax;
        JsValue::from_serde(&serde_json::json!({
            "total_tax": tax,
            "net_amount": net
        })).unwrap()
    }

    // ───── PDA GETTERS ─────
    #[wasm_bindgen]
    pub fn get_vault_pda(&self, user: &str) -> String {
        let user_pk = Pubkey::from_str(user).unwrap_or_default();
        get_vault_pda(&user_pk).to_string()
    }

    #[wasm_bindgen]
    pub fn get_meme_pda(&self, name: &str, spmp_mint: &str, bump: u8) -> String {
        get_meme_pda(name.as_bytes(), spmp_mint.as_bytes(), bump).to_string()
    }
}
