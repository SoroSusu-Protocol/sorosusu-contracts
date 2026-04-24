use soroban_sdk::{contractimpl, Env, Symbol};

pub struct FxOracle;

/// SEP-38 Cross-Border FX Rate Oracle consumer
#[contractimpl]
impl FxOracle {
    /// Store an FX rate for a given asset/fiat pair
    pub fn set_rate(env: Env, asset_id: Symbol, fiat_code: Symbol, rate: i128) {
        let key = (asset_id, fiat_code);
        env.storage().set(&key, &rate);
    }

    /// Get fiat equivalent of an asset amount
    pub fn get_fiat_equivalent(env: Env, asset_id: Symbol, fiat_code: Symbol, amount: i128) -> i128 {
        let key = (asset_id.clone(), fiat_code.clone());
        let rate: i128 = env.storage().get(&key).unwrap_or(0);
        (amount * rate) / 1_000_000 // assume rate scaled by 1e6
    }

    /// Helper: get stored rate
    pub fn get_rate(env: Env, asset_id: Symbol, fiat_code: Symbol) -> i128 {
        let key = (asset_id, fiat_code);
        env.storage().get(&key).unwrap_or(0)
    }
}
