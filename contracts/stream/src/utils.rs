// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]

use soroban_sdk::{Address, Bytes, Env};

/// Get the native XLM SAC (Stellar Asset Contract) address.
///
/// Native XLM on Stellar is wrapped as a SAC with a specific address derived
/// from the native asset ID. This utility looks up the native XLM SAC address
/// programmatically without hardcoding it, making it portable across networks.
///
/// # Parameters
/// - `env` — Soroban environment
///
/// # Returns
/// The `Address` of the native XLM SAC contract.
///
/// # Example
/// ```ignore
/// let native_xlm = get_native_xlm_sac(&env);
/// let token_client = token::Client::new(&env, &native_xlm);
/// ```
pub fn get_native_xlm_sac(env: &Env) -> Address {
    // XDR of `Asset::Native` is the 4-byte discriminant 0. The SAC address is
    // derived deterministically from it and the network passphrase.
    let native_asset_xdr = Bytes::from_array(env, &[0u8, 0, 0, 0]);
    env.deployer()
        .with_stellar_asset(native_asset_xdr)
        .deployed_address()
}

/// Cached cliff/schedule parameters for a single stream.
///
/// Reading cliff and schedule values from storage is expensive; this struct
/// lets callers read them once and reuse the values across repeated
/// `claimable_amount` computations (e.g. inside `withdraw_all`).
#[derive(Clone, Copy)]
pub struct StreamSchedule {
    /// Timestamp at which the cliff ends (0 if no cliff).
    pub cliff: u64,
    /// Timestamp at which accrual starts.
    pub accrual_start: u64,
    /// Timestamp at which the stream ends.
    pub end: u64,
    /// Total amount streamed over the full duration.
    pub total: i128,
}

impl StreamSchedule {
    /// Compute the claimable amount for `now` using the cached schedule values.
    ///
    /// This performs no storage reads; all inputs are held in the struct so
    /// repeated calls (e.g. per stream in `withdraw_all`) avoid redundant
    /// cliff/schedule lookups while producing identical results.
    pub fn claimable_amount(&self, now: u64) -> i128 {
        if now < self.cliff || now < self.accrual_start {
            return 0;
        }
        if now >= self.end {
            return self.total;
        }
        let elapsed = (now - self.accrual_start) as i128;
        let duration = (self.end - self.accrual_start) as i128;
        if duration <= 0 {
            return self.total;
        }
        self.total * elapsed / duration
    }
}

#[cfg(test)]
mod tests {
    use soroban_sdk::Env;

    use super::*;

    #[test]
    fn test_get_native_xlm_sac_returns_valid_address() {
        let env = Env::default();
        let native_xlm = get_native_xlm_sac(&env);
        // Verify it returns an Address (any valid address from StellarAssetClient::native)
        // The function should not panic and return a valid contract address.
        // On-chain, this would resolve to the actual native XLM SAC address.
        let _client = soroban_sdk::token::Client::new(&env, &native_xlm);
        // If we got here, the address is valid and usable
    }

    #[test]
    fn test_stream_schedule_claimable_amount() {
        let schedule = StreamSchedule {
            cliff: 100,
            accrual_start: 100,
            end: 200,
            total: 1000,
        };
        assert_eq!(schedule.claimable_amount(50), 0);
        assert_eq!(schedule.claimable_amount(100), 0);
        assert_eq!(schedule.claimable_amount(150), 500);
        assert_eq!(schedule.claimable_amount(200), 1000);
        assert_eq!(schedule.claimable_amount(250), 1000);
    }
}
