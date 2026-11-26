// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]

use crate::types::{
    ERR_AMOUNT_NOT_POSITIVE, ERR_BELOW_MIN_DEPOSIT, ERR_CLIFF_AFTER_STOP, ERR_INVALID_RATE, ERR_STOP_TIME_PAST,
    ERR_ZERO_DEPOSIT, ERR_ZERO_RATE, ERR_STOP_TIME_TOO_FAR, MAX_STOP_TIME_OFFSET,
};
use soroban_sdk::{Address, Map, Symbol};

/// Maximum number of entries allowed in a single batch operation.
pub const MAX_BATCH_SIZE: u32 = 50;

/// Maximum allowed rate_per_second (1 billion tokens/s — prevents overflow in
/// claimable_amount for any realistic elapsed time up to ~292 years).
pub const MAX_RATE_PER_SECOND: i128 = 1_000_000_000_i128;

/// Validate stream creation parameters.
///
/// # Panics
/// - E002 if `deposit` ≤ 0
/// - E007 if `deposit` < `min_deposit`
/// - E001 if `rate_per_second` ≤ 0
/// - E008 if `rate_per_second` > MAX_RATE_PER_SECOND
/// - E022 if `stop_time` is non-zero and in the past
/// - E030 if `stop_time` exceeds 10-year window from now
/// - if `employer` == `employee`
pub fn validate_create_stream(
    deposit: i128,
    min_deposit: i128,
    rate_per_second: i128,
    stop_time: u64,
    now: u64,
    employer: &Address,
    employee: &Address,
) {
    assert!(deposit > 0, "{}", ERR_ZERO_DEPOSIT);
    assert!(deposit >= min_deposit, "{}", ERR_BELOW_MIN_DEPOSIT);
    assert!(rate_per_second > 0, "{}", ERR_ZERO_RATE);
    assert!(
        rate_per_second <= MAX_RATE_PER_SECOND,
        "{}",
        ERR_INVALID_RATE
    );
    if stop_time > 0 {
        assert!(stop_time > now, "{}", ERR_STOP_TIME_PAST);
        // Enforce maximum 10-year window from now
        assert!(
            stop_time <= now.saturating_add(MAX_STOP_TIME_OFFSET),
            "{}",
            ERR_STOP_TIME_TOO_FAR
        );
    }
    assert!(employer != employee, "employer and employee must differ");
}

/// Validate that `stop_time` does not exceed the maximum 10-year window from `now`.
///
/// A `stop_time` of 0 means "no stop time" and is always allowed.
///
/// # Panics
/// - E030 if `stop_time` is non-zero and exceeds `now + MAX_STOP_TIME_OFFSET`
pub fn validate_stop_time_cap(stop_time: u64, now: u64) {
    if stop_time > 0 {
        assert!(
            stop_time <= now.saturating_add(MAX_STOP_TIME_OFFSET),
            "{}",
            ERR_STOP_TIME_TOO_FAR
        );
    }
}

/// Validate an optional cliff against the stream's stop time (PROD-17).
///
/// # Panics
/// - E032 if `stop_time` is non-zero and `cliff_time > stop_time`
pub fn validate_cliff(cliff_time: Option<u64>, stop_time: u64) {
    if let Some(cliff) = cliff_time {
        if stop_time > 0 {
            assert!(cliff <= stop_time, "{}", ERR_CLIFF_AFTER_STOP);
        }
    }
}

/// Validate a top-up amount.
pub fn validate_top_up(amount: i128) {
    assert!(amount > 0, "{}", ERR_AMOUNT_NOT_POSITIVE);
}

/// Maximum number of entries allowed in a stream metadata map (#467).
pub const MAX_METADATA_ENTRIES: u32 = 8;

/// Maximum length (in bytes) of a metadata key symbol (#467).
pub const MAX_METADATA_KEY_LEN: u32 = 32;

/// Maximum length (in bytes) of a metadata value symbol (#467).
pub const MAX_METADATA_VALUE_LEN: u32 = 64;

/// Validate a stream metadata map (#467).
///
/// # Panics
/// - E040 if the map has more than `MAX_METADATA_ENTRIES` entries
/// - E041 if any key exceeds `MAX_METADATA_KEY_LEN` bytes
/// - E042 if any value exceeds `MAX_METADATA_VALUE_LEN` bytes
pub fn validate_metadata(metadata: &Map<Symbol, Symbol>) {
    use crate::types::{
        ERR_METADATA_KEY_TOO_LONG, ERR_METADATA_TOO_MANY, ERR_METADATA_VALUE_TOO_LONG,
    };

    assert!(
        metadata.len() <= MAX_METADATA_ENTRIES,
        "{}",
        ERR_METADATA_TOO_MANY
    );
    for (key, value) in metadata.iter() {
        assert!(
            key.len() <= MAX_METADATA_KEY_LEN,
            "{}",
            ERR_METADATA_KEY_TOO_LONG
        );
        assert!(
            value.len() <= MAX_METADATA_VALUE_LEN,
            "{}",
            ERR_METADATA_VALUE_TOO_LONG
        );
    }
}

/// Validate a configurable max batch size (#483).
///
/// # Panics
/// - E043 if `max` is outside the allowed range `1..=200`
pub fn validate_max_batch_size(max: u32) {
    use crate::types::{ERR_MAX_BATCH_SIZE_INVALID, MAX_BATCH_SIZE_LOWER, MAX_BATCH_SIZE_UPPER};

    assert!(
        max >= MAX_BATCH_SIZE_LOWER && max <= MAX_BATCH_SIZE_UPPER,
        "{}",
        ERR_MAX_BATCH_SIZE_INVALID
    );
}

/// Validate a new rate passed to `update_rate`.
///
/// Applies the same rules as `create_stream`:
/// - E001 if `rate` ≤ 0
/// - E008 if `rate` > MAX_RATE_PER_SECOND
pub fn validate_rate(rate: i128) {
    assert!(rate > 0, "{}", ERR_ZERO_RATE);
    assert!(rate <= MAX_RATE_PER_SECOND, "{}", ERR_INVALID_RATE);
}

/// Validate the immutable (non-cancellable / non-pausable) stream flag (#445).
///
/// The flag itself is a plain `bool` and needs no range checking; this helper
/// exists to document the semantics enforced elsewhere:
/// - when `true`, `cancel_stream` and `pause_stream` must reject the stream
/// - top-ups and employee withdrawals remain allowed
pub fn validate_immutable_flag(_immutable: bool) {
    // No-op: the flag is a boolean and carries no invalid state.
}

/// Validate a milestone vesting schedule (#446).
///
/// `schedule` is a list of `(timestamp, fraction_bps)` entries where
/// `fraction_bps` is the cumulative share (in basis points) vested at that
/// timestamp. Entries must be strictly increasing in time, each fraction must
/// be in `1..=10000`, and the final fraction must equal `10000` (100%).
///
/// # Panics
/// - E001 if the schedule is empty
/// - E001 if timestamps are not strictly increasing
/// - E001 if any fraction is outside `1..=10000`
/// - E001 if the total (last) fraction is not exactly `10000`
/// - E032 if the first timestamp is before `cliff`
/// - E022 if the first timestamp is before `now`
/// - E032 if `stop_time` is non-zero and the last timestamp exceeds it
pub fn validate_milestone_schedule(
    schedule: &soroban_sdk::Vec<(u64, u32)>,
    cliff: Option<u64>,
    stop_time: u64,
    now: u64,
) {
    assert!(!schedule.is_empty(), "{}", ERR_INVALID_RATE);

    let mut prev_ts: Option<u64> = None;
    let mut last_fraction: u32 = 0;
    for (ts, fraction) in schedule.iter() {
        if let Some(prev) = prev_ts {
            assert!(ts > prev, "{}", ERR_INVALID_RATE);
        }
        assert!(
            fraction >= 1 && fraction <= 10_000,
            "{}",
            ERR_INVALID_RATE
        );
        prev_ts = Some(ts);
        last_fraction = fraction;
    }

    assert!(last_fraction == 10_000, "{}", ERR_INVALID_RATE);

    let first_ts = schedule.get(0).unwrap().0;
    assert!(first_ts >= now, "{}", ERR_STOP_TIME_PAST);
    if let Some(cliff_time) = cliff {
        assert!(first_ts >= cliff_time, "{}", ERR_CLIFF_AFTER_STOP);
    }
    if stop_time > 0 {
        let last_ts = schedule.get(schedule.len() - 1).unwrap().0;
        assert!(last_ts <= stop_time, "{}", ERR_CLIFF_AFTER_STOP);
    }
}

/// Validate a forward rate schedule (#447).
///
/// `schedule` is a sorted list of `(timestamp, rate_per_second)` entries. Each
/// entry defines the active rate from its timestamp until the next entry (the
/// last entry applies indefinitely). Timestamps must be strictly increasing
/// (no overlaps) and the first entry must not be in the past.
///
/// # Panics
/// - E001 if the schedule is empty
/// - E001 if timestamps are not strictly increasing
/// - E001 if any rate is ≤ 0 or > MAX_RATE_PER_SECOND
/// - E022 if the first timestamp is before `now`
/// - E022 if `stop_time` is non-zero and the last timestamp exceeds it
pub fn validate_rate_schedule(
    schedule: &soroban_sdk::Vec<(u64, i128)>,
    now: u64,
    stop_time: u64,
) {
    assert!(!schedule.is_empty(), "{}", ERR_INVALID_RATE);

    let mut prev_ts: Option<u64> = None;
    for (ts, rate) in schedule.iter() {
        if let Some(prev) = prev_ts {
            assert!(ts > prev, "{}", ERR_INVALID_RATE);
        }
        assert!(rate > 0, "{}", ERR_ZERO_RATE);
        assert!(rate <= MAX_RATE_PER_SECOND, "{}", ERR_INVALID_RATE);
        prev_ts = Some(ts);
    }

    let first_ts = schedule.get(0).unwrap().0;
    assert!(first_ts >= now, "{}", ERR_STOP_TIME_PAST);
    if stop_time > 0 {
        let last_ts = schedule.get(schedule.len() - 1).unwrap().0;
        assert!(last_ts <= stop_time, "{}", ERR_STOP_TIME_PAST);
    }
}

/// Validate a `withdraw_to` delegate recipient (#448).
///
/// The delegate may differ from the holder (e.g. a savings wallet) but must be
/// a valid, non-zero address. Withdrawing to the holder themselves is allowed.
///
/// # Panics
/// - E001 if `to` equals the zero address
pub fn validate_withdraw_to(employee: &Address, to: &Address) {
    let _ = employee;
    assert!(
        to != &Address::from_str(&soroban_sdk::Env::default(), ""),
        "{}",
        ERR_AMOUNT_NOT_POSITIVE
    );
}

/// Validate split stream beneficiary configuration.
///
/// # Panics
/// - E031 if fewer than 2 beneficiaries
/// - E032 if more than 20 beneficiaries
/// - E030 if basis points do not sum to exactly 10000 (100%)
pub fn validate_split_beneficiaries(
    beneficiaries: &soroban_sdk::Vec<crate::types::SplitBeneficiary>,
) {
    use crate::types::{
        ERR_SPLIT_BASIS_INVALID, ERR_SPLIT_TOO_FEW, ERR_SPLIT_TOO_MANY, MAX_SPLIT_BENEFICIARIES,
    };

    let count = beneficiaries.len();
    assert!(count >= 2, "{}", ERR_SPLIT_TOO_FEW);
    assert!(count <= MAX_SPLIT_BENEFICIARIES, "{}", ERR_SPLIT_TOO_MANY);
    
    let mut total_bp: u64 = 0;
    for benef in beneficiaries.iter() {
        total_bp += benef.basis_points as u64;
    }
    assert!(total_bp == 10000, "{}", ERR_SPLIT_BASIS_INVALID);
}
