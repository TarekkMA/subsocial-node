//! Autogenerated weights for pallet_free_calls
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-01-27, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./scripts/../target/release/subsocial-node
// benchmark
// --chain
// dev
// --execution
// wasm
// --wasm-execution
// Compiled
// --pallet
// pallet_free_calls
// --extrinsic
// *
// --steps
// 50
// --repeat
// 20
// --heap-pages
// 4096
// --output
// ./pallets/free-calls/src/weights.rs
// --template
// ./.maintain/weight-template.hbs


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_free_calls.
pub trait WeightInfo {
    fn try_free_call() -> Weight;
}

/// Weights for pallet_free_calls using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
        impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
            // Storage: unknown [0xca15211defb6ae0af15535cfecffe8c11afdac6006e1f5e457c882416ea1d17a] (r:1 w:0)
            // Storage: FreeCalls WindowStatsByAccount (r:3 w:3)
        fn try_free_call() -> Weight {
        (101_000_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(4 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
        }
    }

    // For backwards compatibility and tests
    impl WeightInfo for () {
            // Storage: unknown [0xca15211defb6ae0af15535cfecffe8c11afdac6006e1f5e457c882416ea1d17a] (r:1 w:0)
            // Storage: FreeCalls WindowStatsByAccount (r:3 w:3)
        fn try_free_call() -> Weight {
        (101_000_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(4 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
        }
    }
