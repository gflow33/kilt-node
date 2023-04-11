// TODO: UPDATE BEFORE RELEASE

// KILT Blockchain – https://botlabs.org
// Copyright (C) 2019-2023 BOTLabs GmbH

// The KILT Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The KILT Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

// If you feel like getting in touch with us, you can do so at info@botlabs.org

//! Autogenerated weights for `pallet_tips`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-04-05, STEPS: `2`, REPEAT: `1`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `rust-2`, CPU: `12th Gen Intel(R) Core(TM) i9-12900K`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/kilt-parachain
// benchmark
// pallet
// --template=.maintain/runtime-weight-template.hbs
// --header=HEADER-GPL
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --steps=2
// --repeat=1
// --chain=dev
// --pallet=pallet-tips
// --extrinsic=*
// --output=./runtimes/peregrine/src/weights/pallet_tips.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_tips`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_tips::WeightInfo for WeightInfo<T> {
	/// Storage: Tips Reasons (r:1 w:1)
	/// Proof Skipped: Tips Reasons (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tips Tips (r:1 w:1)
	/// Proof Skipped: Tips Tips (max_values: None, max_size: None, mode: Measured)
	/// The range of component `r` is `[0, 16384]`.
	fn report_awesome(_r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4`
		//  Estimated: `4958`
		// Minimum execution time: 19_739_000 picoseconds.
		Weight::from_parts(38_021_000, 0)
			.saturating_add(Weight::from_parts(0, 4958))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: Tips Tips (r:1 w:1)
	/// Proof Skipped: Tips Tips (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tips Reasons (r:0 w:1)
	/// Proof Skipped: Tips Reasons (max_values: None, max_size: None, mode: Measured)
	fn retract_tip() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `253`
		//  Estimated: `2981`
		// Minimum execution time: 17_722_000 picoseconds.
		Weight::from_parts(17_722_000, 0)
			.saturating_add(Weight::from_parts(0, 2981))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: TipsMembership Members (r:1 w:0)
	/// Proof: TipsMembership Members (max_values: Some(1), max_size: Some(3202), added: 3697, mode: MaxEncodedLen)
	/// Storage: Tips Reasons (r:1 w:1)
	/// Proof Skipped: Tips Reasons (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tips Tips (r:0 w:1)
	/// Proof Skipped: Tips Tips (max_values: None, max_size: None, mode: Measured)
	/// The range of component `r` is `[0, 16384]`.
	/// The range of component `t` is `[1, 100]`.
	fn tip_new(r: u32, t: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `171 + t * (32 ±0)`
		//  Estimated: `6516 + t * (64 ±0)`
		// Minimum execution time: 36_187_000 picoseconds.
		Weight::from_parts(28_815_787, 0)
			.saturating_add(Weight::from_parts(0, 6516))
			// Standard Error: 167
			.saturating_add(Weight::from_parts(586, 0).saturating_mul(r.into()))
			// Standard Error: 27_756
			.saturating_add(Weight::from_parts(73_712, 0).saturating_mul(t.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 64).saturating_mul(t.into()))
	}
	/// Storage: TipsMembership Members (r:1 w:0)
	/// Proof: TipsMembership Members (max_values: Some(1), max_size: Some(3202), added: 3697, mode: MaxEncodedLen)
	/// Storage: Tips Tips (r:1 w:1)
	/// Proof Skipped: Tips Tips (max_values: None, max_size: None, mode: Measured)
	/// The range of component `t` is `[1, 100]`.
	fn tip(_t: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `428 + t * (80 ±0)`
		//  Estimated: `14604`
		// Minimum execution time: 23_212_000 picoseconds.
		Weight::from_parts(42_773_000, 0)
			.saturating_add(Weight::from_parts(0, 14604))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: Tips Tips (r:1 w:1)
	/// Proof Skipped: Tips Tips (max_values: None, max_size: None, mode: Measured)
	/// Storage: TipsMembership Members (r:1 w:0)
	/// Proof: TipsMembership Members (max_values: Some(1), max_size: Some(3202), added: 3697, mode: MaxEncodedLen)
	/// Storage: System Account (r:2 w:2)
	/// Proof: System Account (max_values: None, max_size: Some(132), added: 2607, mode: MaxEncodedLen)
	/// Storage: Tips Reasons (r:0 w:1)
	/// Proof Skipped: Tips Reasons (max_values: None, max_size: None, mode: Measured)
	/// The range of component `t` is `[1, 100]`.
	fn close_tip(_t: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1311 + t * (79 ±0)`
		//  Estimated: `29936`
		// Minimum execution time: 72_183_000 picoseconds.
		Weight::from_parts(77_488_000, 0)
			.saturating_add(Weight::from_parts(0, 29936))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: Tips Tips (r:1 w:1)
	/// Proof Skipped: Tips Tips (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tips Reasons (r:0 w:1)
	/// Proof Skipped: Tips Reasons (max_values: None, max_size: None, mode: Measured)
	/// The range of component `t` is `[1, 100]`.
	fn slash_tip(_t: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `301`
		//  Estimated: `3077`
		// Minimum execution time: 22_340_000 picoseconds.
		Weight::from_parts(23_510_000, 0)
			.saturating_add(Weight::from_parts(0, 3077))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(2))
	}
}

#[cfg(test)]
mod tests {
	#[test]
	fn test_report_awesome() {
		assert!(
			<crate::Runtime as frame_system::Config>::BlockWeights::get()
				.per_class
				.get(frame_support::dispatch::DispatchClass::Normal)
				.max_extrinsic
				.unwrap_or_else(<sp_weights::Weight as sp_runtime::traits::Bounded>::max_value)
				.proof_size()
				> 4958
		);
	}
	#[test]
	fn test_retract_tip() {
		assert!(
			<crate::Runtime as frame_system::Config>::BlockWeights::get()
				.per_class
				.get(frame_support::dispatch::DispatchClass::Normal)
				.max_extrinsic
				.unwrap_or_else(<sp_weights::Weight as sp_runtime::traits::Bounded>::max_value)
				.proof_size()
				> 2981
		);
	}
	#[test]
	fn test_tip_new() {
		assert!(
			<crate::Runtime as frame_system::Config>::BlockWeights::get()
				.per_class
				.get(frame_support::dispatch::DispatchClass::Normal)
				.max_extrinsic
				.unwrap_or_else(<sp_weights::Weight as sp_runtime::traits::Bounded>::max_value)
				.proof_size()
				> 6516
		);
	}
	#[test]
	fn test_tip() {
		assert!(
			<crate::Runtime as frame_system::Config>::BlockWeights::get()
				.per_class
				.get(frame_support::dispatch::DispatchClass::Normal)
				.max_extrinsic
				.unwrap_or_else(<sp_weights::Weight as sp_runtime::traits::Bounded>::max_value)
				.proof_size()
				> 14604
		);
	}
	#[test]
	fn test_close_tip() {
		assert!(
			<crate::Runtime as frame_system::Config>::BlockWeights::get()
				.per_class
				.get(frame_support::dispatch::DispatchClass::Normal)
				.max_extrinsic
				.unwrap_or_else(<sp_weights::Weight as sp_runtime::traits::Bounded>::max_value)
				.proof_size()
				> 29936
		);
	}
	#[test]
	fn test_slash_tip() {
		assert!(
			<crate::Runtime as frame_system::Config>::BlockWeights::get()
				.per_class
				.get(frame_support::dispatch::DispatchClass::Normal)
				.max_extrinsic
				.unwrap_or_else(<sp_weights::Weight as sp_runtime::traits::Bounded>::max_value)
				.proof_size()
				> 3077
		);
	}
}
