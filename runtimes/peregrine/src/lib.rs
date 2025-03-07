// KILT Blockchain – https://botlabs.org
// Copyright (C) 2019-2024 BOTLabs GmbH

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

//! The KILT runtime. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use frame_support::{
	construct_runtime,
	genesis_builder_helper::{build_config, create_default_config},
	parameter_types,
	traits::{
		fungible::HoldConsideration,
		tokens::{PayFromAccount, UnityAssetBalanceConversion},
		AsEnsureOriginWithArg, ConstU32, EitherOfDiverse, EnqueueWithOrigin, Everything, InstanceFilter,
		LinearStoragePrice, PrivilegeCmp,
	},
	weights::{ConstantMultiplier, Weight},
};
use frame_system::{pallet_prelude::BlockNumberFor, EnsureRoot, EnsureSigned};
use pallet_asset_switch::xcm::{AccountId32ToAccountId32JunctionConverter, MatchesSwitchPairXcmFeeFungibleAsset};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use sp_api::impl_runtime_apis;
use sp_core::{ConstBool, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{AccountIdLookup, BlakeTwo256, Block as BlockT, ConvertInto, OpaqueKeys},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, Perbill, Permill, RuntimeDebug,
};
use sp_std::{cmp::Ordering, prelude::*};
use sp_version::RuntimeVersion;
use xcm::{v4::Location, VersionedAssetId};
use xcm_builder::{FungiblesAdapter, NoChecking};

use delegation::DelegationAc;
use kilt_support::traits::ItemFilter;
use pallet_did_lookup::linkable_account::LinkableAccountId;
pub use parachain_staking::InflationInfo;
pub use public_credentials;

use runtime_common::{
	asset_switch::{runtime_api::Error as AssetSwitchApiError, EnsureRootAsTreasury},
	assets::{AssetDid, PublicCredentialsFilter},
	authorization::{AuthorizationId, PalletAuthorize},
	constants::{
		self, UnvestedFundsAllowedWithdrawReasons, BLOCK_PROCESSING_VELOCITY, EXISTENTIAL_DEPOSIT, KILT,
		RELAY_CHAIN_SLOT_DURATION_MILLIS, SLOT_DURATION, UNINCLUDED_SEGMENT_CAPACITY,
	},
	dip::merkle::{CompleteMerkleProof, DidMerkleProofOf, DidMerkleRootGenerator},
	errors::PublicCredentialsApiError,
	fees::{ToAuthorCredit, WeightToFee},
	pallet_id,
	xcm_config::RelayOrigin,
	AccountId, AuthorityId, Balance, BlockHashCount, BlockLength, BlockNumber, BlockWeights, DidIdentifier, FeeSplit,
	Hash, Header, Nonce, SendDustAndFeesToTreasury, Signature, SlowAdjustingFeeUpdate,
};

use crate::xcm_config::{LocationToAccountIdConverter, XcmRouter};

#[cfg(feature = "std")]
use sp_version::NativeVersion;

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

#[cfg(feature = "runtime-benchmarks")]
use {kilt_support::signature::AlwaysVerify, runtime_common::benchmarks::DummySignature};

#[cfg(feature = "try-runtime")]
use frame_try_runtime::UpgradeCheckSelect;

#[cfg(test)]
mod tests;

mod dip;
mod weights;
pub mod xcm_config;

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

/// This runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("mashnet-node"),
	impl_name: create_runtime_str!("mashnet-node"),
	authoring_version: 4,
	spec_version: 11500,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 8,
	state_version: 0,
};

/// The version information used to identify this runtime when compiled
/// natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u8 = 38;
}

impl frame_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in
	/// dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The nonce type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The block type as expected in this runtime
	type Block = Block;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeTask = RuntimeTask;
	/// Maximum number of block number to block hash mappings to keep (oldest
	/// pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// Converts a module to an index of this module in the runtime.
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = weights::rocksdb_weights::constants::RocksDbWeight;
	type BaseCallFilter = Everything;
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
	type BlockWeights = BlockWeights;
	type BlockLength = BlockLength;
	type SS58Prefix = SS58Prefix;
	/// The set code logic
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Runtime>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = constants::SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ExistentialDeposit: u128 = EXISTENTIAL_DEPOSIT;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
	pub const MaxFreezes: u32 = 50;
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = constants::multisig::DepositBase;
	type DepositFactor = constants::multisig::DepositFactor;
	type MaxSignatories = constants::multisig::MaxSignitors;
	type WeightInfo = weights::pallet_multisig::WeightInfo<Runtime>;
}

impl pallet_migration::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type MaxMigrationsPerPallet = constants::pallet_migration::MaxMigrationsPerPallet;
	type WeightInfo = weights::pallet_migration::WeightInfo<Runtime>;
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = Nonce;
	type Currency = pallet_balances::Pallet<Runtime>;
	type Deposit = constants::IndicesDeposit;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_indices::WeightInfo<Runtime>;
}

impl pallet_balances::Config for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	type FreezeIdentifier = RuntimeFreezeReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MaxFreezes = MaxFreezes;

	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = runtime_common::SendDustAndFeesToTreasury<Runtime>;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<
		Balances,
		FeeSplit<Runtime, SendDustAndFeesToTreasury<Runtime>, ToAuthorCredit<Runtime>>,
	>;
	type OperationalFeeMultiplier = constants::fee::OperationalFeeMultiplier;
	type WeightToFee = WeightToFee<Runtime>;
	type LengthToFee = ConstantMultiplier<Balance, constants::fee::TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_sudo::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = constants::MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = constants::MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
}

type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
	Runtime,
	RELAY_CHAIN_SLOT_DURATION_MILLIS,
	BLOCK_PROCESSING_VELOCITY,
	UNINCLUDED_SEGMENT_CAPACITY,
>;

impl cumulus_pallet_parachain_system::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpQueue = EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
	type ConsensusHook = ConsensusHook;
	type WeightInfo = weights::cumulus_pallet_parachain_system::WeightInfo<Runtime>;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	pub const MaxAuthorities: u32 = constants::staking::MAX_CANDIDATES;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuthorityId;
	//TODO: handle disabled validators
	type DisabledValidators = ();
	type MaxAuthorities = MaxAuthorities;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
}

parameter_types! {
	pub const UncleGenerations: u32 = 0;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = ParachainStaking;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = ParachainStaking;
	type NextSessionRotation = ParachainStaking;
	type SessionManager = ParachainStaking;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = weights::pallet_session::WeightInfo<Runtime>;
}

impl pallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type BlockNumberProvider = System;
	// disable vested transfers by setting min amount to max balance
	type MinVestedTransfer = constants::MinVestedTransfer;
	type WeightInfo = weights::pallet_vesting::WeightInfo<Runtime>;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	const MAX_VESTING_SCHEDULES: u32 = constants::MAX_VESTING_SCHEDULES;
}

parameter_types! {
	pub const MaxClaims: u32 = 50;
	pub const UsableBalance: Balance = KILT;
	pub const AutoUnlockBound: u32 = 100;
	pub const PreImageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = weights::pallet_preimage::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreImageHoldReason,
		LinearStoragePrice<constants::preimage::PreimageBaseDeposit, constants::ByteDeposit, Balance>,
	>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const NoPreimagePostponement: Option<BlockNumber> = Some(10);
}

type ScheduleOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
>;

/// Used the compare the privilege of an origin inside the scheduler.
pub struct OriginPrivilegeCmp;

impl PrivilegeCmp<OriginCaller> for OriginPrivilegeCmp {
	fn cmp_privilege(left: &OriginCaller, right: &OriginCaller) -> Option<Ordering> {
		if left == right {
			return Some(Ordering::Equal);
		}

		match (left, right) {
			// Root is greater than anything.
			(OriginCaller::system(frame_system::RawOrigin::Root), _) => Some(Ordering::Greater),
			// Check which one has more yes votes.
			(
				OriginCaller::Council(pallet_collective::RawOrigin::Members(l_yes_votes, l_count)),
				OriginCaller::Council(pallet_collective::RawOrigin::Members(r_yes_votes, r_count)),
			) => Some((l_yes_votes * r_count).cmp(&(r_yes_votes * l_count))),
			// For every other origin we don't care, as they are not used for `ScheduleOrigin`.
			_ => None,
		}
	}
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = ScheduleOrigin;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = weights::pallet_scheduler::WeightInfo<Runtime>;
	type OriginPrivilegeCmp = OriginPrivilegeCmp;
	type Preimages = Preimage;
}

parameter_types! {
	pub const InstantAllowed: bool = true;
	pub const MaxVotes: u32 = 100;
	pub const MaxProposals: u32 = 100;
}

impl pallet_democracy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EnactmentPeriod = constants::governance::EnactmentPeriod;
	type VoteLockingPeriod = constants::governance::VotingPeriod;
	type LaunchPeriod = constants::governance::LaunchPeriod;
	type VotingPeriod = constants::governance::VotingPeriod;
	type MinimumDeposit = constants::governance::MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
	/// A majority can have the next scheduled referendum be a straight
	/// majority-carries vote.
	type ExternalMajorityOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
	/// A unanimous council can have the next scheduled referendum be a straight
	/// default-carries (NTB) vote.
	type ExternalDefaultOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
	/// Two thirds of the technical committee can have an
	/// ExternalMajority/ExternalDefault vote be tabled immediately and with a
	/// shorter voting/enactment period.
	type FastTrackOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>;
	type InstantOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;
	type InstantAllowed = InstantAllowed;
	type FastTrackVotingPeriod = constants::governance::FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to
	// it.
	type CancellationOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>,
	>;
	// To cancel a proposal before it has been passed, the technical committee must
	// be unanimous or Root must agree.
	type CancelProposalOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
	>;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// Any single technical committee member may veto a coming council proposal,
	// however they can only do it once and it lasts only for the cooloff period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type CooloffPeriod = constants::governance::CooloffPeriod;
	type Slash = Treasury;
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxVotes = MaxVotes;
	type WeightInfo = weights::pallet_democracy::WeightInfo<Runtime>;
	type MaxProposals = MaxProposals;
	type Preimages = Preimage;
	type MaxDeposits = ConstU32<100>;
	type MaxBlacklisted = ConstU32<100>;
	type SubmitOrigin = EnsureSigned<AccountId>;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 20 * KILT;
	pub const SpendPeriod: BlockNumber = constants::governance::SPEND_PERIOD;
	pub const Burn: Permill = Permill::zero();
	pub const MaxApprovals: u32 = 100;
	pub MaxProposalWeight: Weight = Perbill::from_percent(50) * BlockWeights::get().max_block;
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

type ApproveOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
>;

type MoreThanHalfCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;

impl pallet_treasury::Config for Runtime {
	type PalletId = pallet_id::Treasury;
	type Currency = Balances;
	type ApproveOrigin = ApproveOrigin;
	type RejectOrigin = MoreThanHalfCouncil;
	type RuntimeEvent = RuntimeEvent;
	type OnSlash = Treasury;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ();
	type SpendPeriod = SpendPeriod;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type SpendOrigin = frame_support::traits::NeverEnsureOrigin<Balance>;
	#[cfg(feature = "runtime-benchmarks")]
	type SpendOrigin = frame_system::EnsureWithSuccess<EnsureRoot<AccountId>, AccountId, benches::MaxBalance>;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = ();
	type WeightInfo = weights::pallet_treasury::WeightInfo<Runtime>;
	type MaxApprovals = MaxApprovals;
	type AssetKind = ();
	type BalanceConverter = UnityAssetBalanceConversion;
	type Beneficiary = AccountId;
	type BeneficiaryLookup = AccountIdLookup<Self::Beneficiary, ()>;
	type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
	type PayoutPeriod = runtime_common::constants::treasury::PayoutPeriod;

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = runtime_common::benchmarks::treasury::BenchmarkHelper<Runtime>;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MaxProposalWeight = MaxProposalWeight;
	type MotionDuration = constants::governance::CouncilMotionDuration;
	type MaxProposals = constants::governance::CouncilMaxProposals;
	type MaxMembers = constants::governance::CouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = weights::pallet_collective_council::WeightInfo<Runtime>;
	type SetMembersOrigin = EnsureRoot<AccountId>;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type MaxProposalWeight = MaxProposalWeight;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = constants::governance::TechnicalMotionDuration;
	type MaxProposals = constants::governance::TechnicalMaxProposals;
	type MaxMembers = constants::governance::TechnicalMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = weights::pallet_collective_technical_committee::WeightInfo<Runtime>;
	type SetMembersOrigin = EnsureRoot<AccountId>;
}

type TechnicalMembershipProvider = pallet_membership::Instance1;
impl pallet_membership::Config<TechnicalMembershipProvider> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = MoreThanHalfCouncil;
	type RemoveOrigin = MoreThanHalfCouncil;
	type SwapOrigin = MoreThanHalfCouncil;
	type ResetOrigin = MoreThanHalfCouncil;
	type PrimeOrigin = MoreThanHalfCouncil;
	type MembershipInitialized = TechnicalCommittee;
	type MembershipChanged = TechnicalCommittee;
	type MaxMembers = constants::governance::TechnicalMaxMembers;
	type WeightInfo = weights::pallet_membership::WeightInfo<Runtime>;
}

type TipsMembershipProvider = pallet_membership::Instance2;
impl pallet_membership::Config<TipsMembershipProvider> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = MoreThanHalfCouncil;
	type RemoveOrigin = MoreThanHalfCouncil;
	type SwapOrigin = MoreThanHalfCouncil;
	type ResetOrigin = MoreThanHalfCouncil;
	type PrimeOrigin = MoreThanHalfCouncil;
	type MembershipInitialized = ();
	type MembershipChanged = ();
	type MaxMembers = constants::governance::TipperMaxMembers;
	type WeightInfo = weights::pallet_membership::WeightInfo<Runtime>;
}

impl pallet_tips::Config for Runtime {
	type MaximumReasonLength = constants::tips::MaximumReasonLength;
	type DataDepositPerByte = constants::ByteDeposit;
	type Tippers = runtime_common::Tippers<Runtime, TipsMembershipProvider>;
	type TipCountdown = constants::tips::TipCountdown;
	type TipFindersFee = constants::tips::TipFindersFee;
	type TipReportDepositBase = constants::tips::TipReportDepositBase;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_tips::WeightInfo<Runtime>;
	type MaxTipAmount = constants::tips::MaxTipAmount;
}

impl attestation::Config for Runtime {
	type EnsureOrigin = did::EnsureDidOrigin<DidIdentifier, AccountId>;
	type OriginSuccess = did::DidRawOrigin<AccountId, DidIdentifier>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::attestation::WeightInfo<Runtime>;

	type Currency = Balances;
	type Deposit = constants::attestation::AttestationDeposit;
	type MaxDelegatedAttestations = constants::attestation::MaxDelegatedAttestations;
	type AttesterId = DidIdentifier;
	type AuthorizationId = AuthorizationId<<Runtime as delegation::Config>::DelegationNodeId>;
	type AccessControl = PalletAuthorize<DelegationAc<Runtime>>;
	type BalanceMigrationManager = Migration;
}

impl delegation::Config for Runtime {
	type DelegationEntityId = DidIdentifier;
	type DelegationNodeId = Hash;

	type EnsureOrigin = did::EnsureDidOrigin<DidIdentifier, AccountId>;
	type OriginSuccess = did::DidRawOrigin<AccountId, DidIdentifier>;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type DelegationSignatureVerification = did::DidSignatureVerify<Runtime>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Signature = did::DidSignature;

	#[cfg(feature = "runtime-benchmarks")]
	type Signature = DummySignature;
	#[cfg(feature = "runtime-benchmarks")]
	type DelegationSignatureVerification = AlwaysVerify<AccountId, Vec<u8>, Self::Signature>;

	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MaxSignatureByteLength = constants::delegation::MaxSignatureByteLength;
	type MaxParentChecks = constants::delegation::MaxParentChecks;
	type MaxRevocations = constants::delegation::MaxRevocations;
	type MaxRemovals = constants::delegation::MaxRemovals;
	type MaxChildren = constants::delegation::MaxChildren;
	type WeightInfo = weights::delegation::WeightInfo<Runtime>;
	type Currency = Balances;
	type Deposit = constants::delegation::DelegationDeposit;
	type BalanceMigrationManager = Migration;
}

impl ctype::Config for Runtime {
	type CtypeCreatorId = AccountId;
	type Currency = Balances;
	type Fee = constants::CtypeFee;
	type FeeCollector = runtime_common::SendDustAndFeesToTreasury<Runtime>;

	type EnsureOrigin = did::EnsureDidOrigin<DidIdentifier, AccountId>;
	type OriginSuccess = did::DidRawOrigin<AccountId, DidIdentifier>;
	type OverarchingOrigin = EnsureRoot<AccountId>;

	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::ctype::WeightInfo<Runtime>;
}

impl did::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeOrigin = RuntimeOrigin;
	type Currency = Balances;
	type DidIdentifier = DidIdentifier;
	type KeyDeposit = constants::did::KeyDeposit;
	type ServiceEndpointDeposit = constants::did::ServiceEndpointDeposit;
	type BaseDeposit = constants::did::DidBaseDeposit;
	type Fee = constants::did::DidFee;
	type FeeCollector = runtime_common::SendDustAndFeesToTreasury<Runtime>;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type EnsureOrigin = did::EnsureDidOrigin<DidIdentifier, AccountId>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type OriginSuccess = did::DidRawOrigin<AccountId, DidIdentifier>;

	#[cfg(feature = "runtime-benchmarks")]
	type EnsureOrigin = EnsureSigned<DidIdentifier>;
	#[cfg(feature = "runtime-benchmarks")]
	type OriginSuccess = DidIdentifier;

	type MaxNewKeyAgreementKeys = constants::did::MaxNewKeyAgreementKeys;
	type MaxTotalKeyAgreementKeys = constants::did::MaxTotalKeyAgreementKeys;
	type MaxPublicKeysPerDid = constants::did::MaxPublicKeysPerDid;
	type MaxBlocksTxValidity = constants::did::MaxBlocksTxValidity;
	type MaxNumberOfServicesPerDid = constants::did::MaxNumberOfServicesPerDid;
	type MaxServiceIdLength = constants::did::MaxServiceIdLength;
	type MaxServiceTypeLength = constants::did::MaxServiceTypeLength;
	type MaxServiceUrlLength = constants::did::MaxServiceUrlLength;
	type MaxNumberOfTypesPerService = constants::did::MaxNumberOfTypesPerService;
	type MaxNumberOfUrlsPerService = constants::did::MaxNumberOfUrlsPerService;
	type WeightInfo = weights::did::WeightInfo<Runtime>;
	type BalanceMigrationManager = Migration;
}

impl pallet_did_lookup::Config for Runtime {
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeEvent = RuntimeEvent;

	type DidIdentifier = DidIdentifier;

	type Currency = Balances;
	type Deposit = constants::did_lookup::DidLookupDeposit;

	type EnsureOrigin = did::EnsureDidOrigin<DidIdentifier, AccountId>;
	type OriginSuccess = did::DidRawOrigin<AccountId, DidIdentifier>;

	type WeightInfo = weights::pallet_did_lookup::WeightInfo<Runtime>;
	type BalanceMigrationManager = Migration;
}

impl pallet_web3_names::Config for Runtime {
	type RuntimeHoldReason = RuntimeHoldReason;
	type BanOrigin = EnsureRoot<AccountId>;
	type OwnerOrigin = did::EnsureDidOrigin<DidIdentifier, AccountId>;
	type OriginSuccess = did::DidRawOrigin<AccountId, DidIdentifier>;
	type Currency = Balances;
	type Deposit = constants::web3_names::Web3NameDeposit;
	type RuntimeEvent = RuntimeEvent;
	type MaxNameLength = constants::web3_names::MaxNameLength;
	type MinNameLength = constants::web3_names::MinNameLength;
	type Web3Name = pallet_web3_names::web3_name::AsciiWeb3Name<Runtime>;
	type Web3NameOwner = DidIdentifier;
	type WeightInfo = weights::pallet_web3_names::WeightInfo<Runtime>;
	type BalanceMigrationManager = Migration;
}

impl pallet_inflation::Config for Runtime {
	type Currency = Balances;
	type InitialPeriodLength = constants::treasury::InitialPeriodLength;
	type InitialPeriodReward = constants::treasury::InitialPeriodReward;
	type Beneficiary = runtime_common::SendDustAndFeesToTreasury<Runtime>;
	type WeightInfo = weights::pallet_inflation::WeightInfo<Runtime>;
}

impl parachain_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MinBlocksPerRound = constants::staking::MinBlocksPerRound;
	type DefaultBlocksPerRound = constants::staking::DefaultBlocksPerRound;
	type StakeDuration = constants::staking::StakeDuration;
	type ExitQueueDelay = constants::staking::ExitQueueDelay;
	type MinCollators = constants::staking::MinCollators;
	type MinRequiredCollators = constants::staking::MinRequiredCollators;
	type MaxDelegationsPerRound = constants::staking::MaxDelegationsPerRound;
	type MaxDelegatorsPerCollator = constants::staking::MaxDelegatorsPerCollator;
	type MinCollatorStake = constants::staking::MinCollatorStake;
	type MinCollatorCandidateStake = constants::staking::MinCollatorStake;
	type MaxTopCandidates = constants::staking::MaxCollatorCandidates;
	type MinDelegatorStake = constants::staking::MinDelegatorStake;
	type MaxUnstakeRequests = constants::staking::MaxUnstakeRequests;
	type NetworkRewardRate = constants::staking::NetworkRewardRate;
	type NetworkRewardStart = constants::staking::NetworkRewardStart;
	type NetworkRewardBeneficiary = runtime_common::SendDustAndFeesToTreasury<Runtime>;
	type WeightInfo = weights::parachain_staking::WeightInfo<Runtime>;

	const BLOCKS_PER_YEAR: BlockNumberFor<Self> = constants::BLOCKS_PER_YEAR;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

impl public_credentials::Config for Runtime {
	type RuntimeHoldReason = RuntimeHoldReason;
	type AccessControl = PalletAuthorize<DelegationAc<Runtime>>;
	type AttesterId = DidIdentifier;
	type AuthorizationId = AuthorizationId<<Runtime as delegation::Config>::DelegationNodeId>;
	type CredentialId = Hash;
	type CredentialHash = BlakeTwo256;
	type Currency = Balances;
	type Deposit = runtime_common::constants::public_credentials::Deposit;
	type EnsureOrigin = did::EnsureDidOrigin<DidIdentifier, AccountId>;
	type MaxEncodedClaimsLength = runtime_common::constants::public_credentials::MaxEncodedClaimsLength;
	type MaxSubjectIdLength = runtime_common::constants::public_credentials::MaxSubjectIdLength;
	type OriginSuccess = did::DidRawOrigin<AccountId, DidIdentifier>;
	type RuntimeEvent = RuntimeEvent;
	type SubjectId = runtime_common::assets::AssetDid;
	type WeightInfo = weights::public_credentials::WeightInfo<Runtime>;
	type BalanceMigrationManager = Migration;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug, MaxEncodedLen, scale_info::TypeInfo,
)]
pub enum ProxyType {
	/// Allow for any call.
	Any,
	/// Allow for calls that do not move tokens out of the caller's account.
	NonTransfer,
	/// Allow for governance-related calls.
	Governance,
	/// Allow for staking-related calls.
	ParachainStaking,
	/// Allow for calls that cancel proxy information.
	CancelProxy,
	/// Allow for calls that do not result in a deposit being claimed (e.g., for
	/// attestations, delegations, or DIDs).
	NonDepositClaiming,
}

impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}

impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => matches!(
				c,
				RuntimeCall::Attestation(..)
					// Excludes `Balances`
					| RuntimeCall::Council(..)
					| RuntimeCall::Ctype(..)
					| RuntimeCall::Delegation(..)
					| RuntimeCall::Democracy(..)
					| RuntimeCall::DepositStorage(..)
					| RuntimeCall::Did(..)
					| RuntimeCall::DidLookup(..)
					| RuntimeCall::DipProvider(..)
					| RuntimeCall::Indices(
						// Excludes `force_transfer`, and `transfer`
						pallet_indices::Call::claim { .. }
							| pallet_indices::Call::free { .. }
							| pallet_indices::Call::freeze { .. }
					)
					| RuntimeCall::Multisig(..)
					| RuntimeCall::ParachainStaking(..)
					// Excludes `ParachainSystem`
					| RuntimeCall::Preimage(..)
					| RuntimeCall::Proxy(..)
					| RuntimeCall::PublicCredentials(..)
					| RuntimeCall::Scheduler(..)
					| RuntimeCall::Session(..)
					| RuntimeCall::System(..)
					| RuntimeCall::TechnicalCommittee(..)
					| RuntimeCall::TechnicalMembership(..)
					| RuntimeCall::TipsMembership(..)
					| RuntimeCall::Timestamp(..)
					| RuntimeCall::Treasury(..)
					| RuntimeCall::Utility(..)
					| RuntimeCall::Vesting(
						// Excludes `force_vested_transfer`, `merge_schedules`, and `vested_transfer`
						pallet_vesting::Call::vest { .. }
							| pallet_vesting::Call::vest_other { .. }
					)
					| RuntimeCall::Web3Names(..),
			),
			ProxyType::NonDepositClaiming => matches!(
				c,
				RuntimeCall::Attestation(
						// Excludes `reclaim_deposit`
						attestation::Call::add { .. }
							| attestation::Call::remove { .. }
							| attestation::Call::revoke { .. }
							| attestation::Call::change_deposit_owner { .. }
							| attestation::Call::update_deposit { .. }
					)
					// Excludes `Balances`
					| RuntimeCall::Council(..)
					| RuntimeCall::Ctype(..)
					| RuntimeCall::Delegation(
						// Excludes `reclaim_deposit`
						delegation::Call::add_delegation { .. }
							| delegation::Call::create_hierarchy { .. }
							| delegation::Call::remove_delegation { .. }
							| delegation::Call::revoke_delegation { .. }
							| delegation::Call::update_deposit { .. }
							| delegation::Call::change_deposit_owner { .. }
					)
					| RuntimeCall::Democracy(..)
					// Excludes `DepositStorage`
					| RuntimeCall::Did(
						// Excludes `reclaim_deposit`
						did::Call::add_key_agreement_key { .. }
							| did::Call::add_service_endpoint { .. }
							| did::Call::create { .. }
							| did::Call::delete { .. }
							| did::Call::remove_attestation_key { .. }
							| did::Call::remove_delegation_key { .. }
							| did::Call::remove_key_agreement_key { .. }
							| did::Call::remove_service_endpoint { .. }
							| did::Call::set_attestation_key { .. }
							| did::Call::set_authentication_key { .. }
							| did::Call::set_delegation_key { .. }
							| did::Call::submit_did_call { .. }
							| did::Call::update_deposit { .. }
							| did::Call::change_deposit_owner { .. }
							| did::Call::create_from_account { .. }
							| did::Call::dispatch_as { .. }
					)
					| RuntimeCall::DidLookup(
						// Excludes `reclaim_deposit`
						pallet_did_lookup::Call::associate_account { .. }
							| pallet_did_lookup::Call::associate_sender { .. }
							| pallet_did_lookup::Call::remove_account_association { .. }
							| pallet_did_lookup::Call::remove_sender_association { .. }
							| pallet_did_lookup::Call::update_deposit { .. }
							| pallet_did_lookup::Call::change_deposit_owner { .. }
					)
					| RuntimeCall::DipProvider(..)
					| RuntimeCall::Indices(..)
					| RuntimeCall::Multisig(..)
					| RuntimeCall::ParachainStaking(..)
					// Excludes `ParachainSystem`
					| RuntimeCall::Preimage(..)
					| RuntimeCall::Proxy(..)
					| RuntimeCall::PublicCredentials(
						// Excludes `reclaim_deposit`
						public_credentials::Call::add { .. }
						| public_credentials::Call::revoke { .. }
						| public_credentials::Call::unrevoke { .. }
						| public_credentials::Call::remove { .. }
						| public_credentials::Call::update_deposit { .. }
						| public_credentials::Call::change_deposit_owner { .. }
					)
					| RuntimeCall::Scheduler(..)
					| RuntimeCall::Session(..)
					// Excludes `Sudo`
					| RuntimeCall::System(..)
					| RuntimeCall::TechnicalCommittee(..)
					| RuntimeCall::TechnicalMembership(..)
					| RuntimeCall::TipsMembership(..)
					| RuntimeCall::Timestamp(..)
					| RuntimeCall::Treasury(..)
					| RuntimeCall::Utility(..)
					| RuntimeCall::Vesting(..)
					| RuntimeCall::Web3Names(
						// Excludes `ban`, and `reclaim_deposit`
						pallet_web3_names::Call::claim { .. }
							| pallet_web3_names::Call::release_by_owner { .. }
							| pallet_web3_names::Call::unban { .. }
							| pallet_web3_names::Call::update_deposit { .. }
							| pallet_web3_names::Call::change_deposit_owner { .. }
					),
			),
			ProxyType::Governance => matches!(
				c,
				RuntimeCall::Council(..)
					| RuntimeCall::Democracy(..)
					| RuntimeCall::TechnicalCommittee(..)
					| RuntimeCall::TechnicalMembership(..)
					| RuntimeCall::TipsMembership(..)
					| RuntimeCall::Treasury(..)
					| RuntimeCall::Utility(..)
			),
			ProxyType::ParachainStaking => {
				matches!(
					c,
					RuntimeCall::ParachainStaking(..) | RuntimeCall::Session(..) | RuntimeCall::Utility(..)
				)
			}
			ProxyType::CancelProxy => matches!(c, RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. })),
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			// "anything" always contains any subset
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			// reclaiming deposits is part of NonTransfer but not in NonDepositClaiming
			(ProxyType::NonDepositClaiming, ProxyType::NonTransfer) => false,
			// everything except NonTransfer and Any is part of NonDepositClaiming
			(ProxyType::NonDepositClaiming, _) => true,
			// Transfers are part of NonDepositClaiming but not in NonTransfer
			(ProxyType::NonTransfer, ProxyType::NonDepositClaiming) => false,
			// everything except NonDepositClaiming and Any is part of NonTransfer
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = constants::proxy::ProxyDepositBase;
	type ProxyDepositFactor = constants::proxy::ProxyDepositFactor;
	type MaxProxies = constants::proxy::MaxProxies;
	type MaxPending = constants::proxy::MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = constants::proxy::AnnouncementDepositBase;
	type AnnouncementDepositFactor = constants::proxy::AnnouncementDepositFactor;
	type WeightInfo = weights::pallet_proxy::WeightInfo<Runtime>;
}

parameter_types! {
	pub CheckingAccount: AccountId = PolkadotXcm::check_account();
}

pub type KiltToEKiltSwitchPallet = pallet_asset_switch::Instance1;
impl pallet_asset_switch::Config<KiltToEKiltSwitchPallet> for Runtime {
	type AccountIdConverter = AccountId32ToAccountId32JunctionConverter;
	type AssetTransactor = FungiblesAdapter<
		Fungibles,
		MatchesSwitchPairXcmFeeFungibleAsset<Runtime, KiltToEKiltSwitchPallet>,
		LocationToAccountIdConverter,
		AccountId,
		NoChecking,
		CheckingAccount,
	>;
	type FeeOrigin = EnsureRoot<AccountId>;
	type LocalCurrency = Balances;
	type PauseOrigin = EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type SubmitterOrigin = EnsureSigned<AccountId>;
	type SwitchHooks = runtime_common::asset_switch::hooks::RestrictSwitchDestinationToSelf;
	type SwitchOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pallet_asset_switch::WeightInfo<Runtime>;
	type XcmRouter = XcmRouter;

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = crate::benches::CreateFungibleForAssetSwitchPool1;
}

// No deposit is taken since creation is permissioned. Only the root origin can
// create new assets, and the owner will be the treasury account.
impl pallet_assets::Config for Runtime {
	type ApprovalDeposit = runtime_common::constants::assets::ApprovalDeposit;
	type AssetAccountDeposit = runtime_common::constants::assets::AssetAccountDeposit;
	type AssetDeposit = runtime_common::constants::assets::AssetDeposit;
	type AssetId = Location;
	type AssetIdParameter = Location;
	type Balance = Balance;
	type CallbackHandle = ();
	type CreateOrigin = AsEnsureOriginWithArg<EnsureRootAsTreasury<Runtime>>;
	type Currency = Balances;
	type Extra = ();
	type ForceOrigin = EnsureRoot<AccountId>;
	type Freezer = ();
	type MetadataDepositBase = runtime_common::constants::assets::MetaDepositBase;
	type MetadataDepositPerByte = runtime_common::constants::assets::MetaDepositPerByte;
	type RemoveItemsLimit = runtime_common::constants::assets::RemoveItemsLimit;
	type RuntimeEvent = RuntimeEvent;
	type StringLimit = runtime_common::constants::assets::StringLimit;
	type WeightInfo = weights::pallet_assets::WeightInfo<Runtime>;

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = runtime_common::asset_switch::NoopBenchmarkHelper;
}

construct_runtime! {
	pub enum Runtime
	{
		System: frame_system = 0,
		// DELETED: RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip = 1,

		Timestamp: pallet_timestamp = 2,
		Indices: pallet_indices exclude_parts { Config } = 5,
		Balances: pallet_balances = 6,
		TransactionPayment: pallet_transaction_payment exclude_parts { Config } = 7,
		Sudo: pallet_sudo = 8,
		// Configuration: pallet_configuration = 9,

		// Consensus support.
		// The following order MUST NOT be changed: Aura -> Session -> Staking -> Authorship -> AuraExt
		// Dependencies: AuraExt on Aura, Authorship and Session on ParachainStaking
		Aura: pallet_aura = 23,
		Session: pallet_session = 22,
		ParachainStaking: parachain_staking = 21,
		Authorship: pallet_authorship = 20,
		AuraExt: cumulus_pallet_aura_ext = 24,

		Democracy: pallet_democracy = 30,
		Council: pallet_collective::<Instance1> = 31,
		TechnicalCommittee: pallet_collective::<Instance2> = 32,
		// reserved: parachain council election = 33,
		TechnicalMembership: pallet_membership::<Instance1> = 34,
		Treasury: pallet_treasury = 35,
		// DELETED: RelayMigration: pallet_relay_migration = 36,
		// DELETED: DynFilter: pallet_dyn_filter = 37,

		// A stateless pallet with helper extrinsics (batch extrinsics, send from different origins, ...)
		Utility: pallet_utility = 40,

		// Vesting. Usable initially, but removed once all vesting is finished.
		Vesting: pallet_vesting = 41,

		Scheduler: pallet_scheduler = 42,

		// Allowing accounts to give permission to other accounts to dispatch types of calls from their signed origin
		Proxy: pallet_proxy = 43,

		// Preimage pallet allows the storage of large bytes blob
		Preimage: pallet_preimage = 44,

		// Tips module to reward contributions to the ecosystem with small amount of KILTs.
		TipsMembership: pallet_membership::<Instance2> = 45,
		Tips: pallet_tips = 46,

		Multisig: pallet_multisig = 47,

		AssetSwitchPool1: pallet_asset_switch::<Instance1> = 48,
		Fungibles: pallet_assets = 49,

		// KILT Pallets. Start indices 60 to leave room
		// DELETED: KiltLaunch: kilt_launch = 60,
		Ctype: ctype = 61,
		Attestation: attestation = 62,
		Delegation: delegation = 63,
		Did: did = 64,
		// DELETED: CrowdloanContributors = 65,
		Inflation: pallet_inflation = 66,
		DidLookup: pallet_did_lookup = 67,
		Web3Names: pallet_web3_names = 68,
		PublicCredentials: public_credentials = 69,
		Migration: pallet_migration = 70,
		DipProvider: pallet_dip_provider = 71,
		DepositStorage: pallet_deposit_storage = 72,

		// Parachains pallets. Start indices at 80 to leave room.

		// Among others: Send and receive DMP and XCMP messages.
		ParachainSystem: cumulus_pallet_parachain_system = 80,
		ParachainInfo: parachain_info = 81,
		// Wrap and unwrap XCMP messages to send and receive them. Queue them for later processing.
		XcmpQueue: cumulus_pallet_xcmp_queue = 82,
		// Build XCM scripts.
		PolkadotXcm: pallet_xcm = 83,
		// Does nothing cool, just provides an origin.
		CumulusXcm: cumulus_pallet_xcm exclude_parts { Call } = 84,
		// DmpQueue: cumulus_pallet_dmp_queue = 85,
		// Queue and pass DMP messages on to be executed.
		MessageQueue: pallet_message_queue = 86,
	}
}

impl did::DeriveDidCallAuthorizationVerificationKeyRelationship for RuntimeCall {
	fn derive_verification_key_relationship(&self) -> did::DeriveDidCallKeyRelationshipResult {
		/// ensure that all calls have the same VerificationKeyRelationship
		fn single_key_relationship(calls: &[RuntimeCall]) -> did::DeriveDidCallKeyRelationshipResult {
			let init = calls
				.get(0)
				.ok_or(did::RelationshipDeriveError::InvalidCallParameter)?
				.derive_verification_key_relationship()?;
			calls
				.iter()
				.skip(1)
				.map(RuntimeCall::derive_verification_key_relationship)
				.try_fold(init, |acc, next| {
					if next.is_err() {
						next
					} else if Ok(acc) == next {
						Ok(acc)
					} else {
						Err(did::RelationshipDeriveError::InvalidCallParameter)
					}
				})
		}
		match self {
			RuntimeCall::Attestation { .. } => Ok(did::DidVerificationKeyRelationship::AssertionMethod),
			RuntimeCall::Ctype { .. } => Ok(did::DidVerificationKeyRelationship::AssertionMethod),
			RuntimeCall::Delegation { .. } => Ok(did::DidVerificationKeyRelationship::CapabilityDelegation),
			RuntimeCall::DipProvider { .. } => Ok(did::DidVerificationKeyRelationship::Authentication),
			// DID creation is not allowed through the DID proxy.
			RuntimeCall::Did(did::Call::create { .. }) => Err(did::RelationshipDeriveError::NotCallableByDid),
			RuntimeCall::Did { .. } => Ok(did::DidVerificationKeyRelationship::Authentication),
			RuntimeCall::Web3Names { .. } => Ok(did::DidVerificationKeyRelationship::Authentication),
			RuntimeCall::PublicCredentials { .. } => Ok(did::DidVerificationKeyRelationship::AssertionMethod),
			RuntimeCall::DidLookup { .. } => Ok(did::DidVerificationKeyRelationship::Authentication),
			RuntimeCall::Utility(pallet_utility::Call::batch { calls }) => single_key_relationship(&calls[..]),
			RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) => single_key_relationship(&calls[..]),
			RuntimeCall::Utility(pallet_utility::Call::force_batch { calls }) => single_key_relationship(&calls[..]),
			#[cfg(not(feature = "runtime-benchmarks"))]
			_ => Err(did::RelationshipDeriveError::NotCallableByDid),
			// By default, returns the authentication key
			#[cfg(feature = "runtime-benchmarks")]
			_ => Ok(did::DidVerificationKeyRelationship::Authentication),
		}
	}

	// Always return a System::remark() extrinsic call
	#[cfg(feature = "runtime-benchmarks")]
	fn get_call_for_did_call_benchmark() -> Self {
		RuntimeCall::System(frame_system::Call::remark { remark: vec![] })
	}
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	// Executes pallet hooks in the order of definition in construct_runtime
	AllPalletsWithSystem,
	frame_support::migrations::RemovePallet<DmpPalletName, <Runtime as frame_system::Config>::DbWeight>,
>;

parameter_types! {
	pub const DmpPalletName: &'static str = "DmpQueue";
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {

	use frame_system::RawOrigin;
	use pallet_asset_switch::PartialBenchmarkInfo;
	use runtime_common::AccountId;
	use xcm::v4::{Asset, AssetId, Fungibility, Junction, Junctions, Location, ParentThen};

	use crate::{Fungibles, ParachainSystem};

	frame_support::parameter_types! {
		pub const MaxBalance: crate::Balance = crate::Balance::max_value();
	}

	frame_benchmarking::define_benchmarks!(
		[frame_system, SystemBench::<Runtime>]
		[pallet_timestamp, Timestamp]
		[pallet_indices, Indices]
		[pallet_balances, Balances]
		[pallet_session, SessionBench::<Runtime>]
		[parachain_staking, ParachainStaking]
		[pallet_democracy, Democracy]
		[pallet_collective, Council]
		[pallet_collective, TechnicalCommittee]
		[pallet_membership, TechnicalMembership]
		[pallet_treasury, Treasury]
		[pallet_sudo, Sudo]
		[pallet_utility, Utility]
		[pallet_vesting, Vesting]
		[pallet_scheduler, Scheduler]
		[pallet_proxy, Proxy]
		[pallet_preimage, Preimage]
		[pallet_tips, Tips]
		[pallet_multisig, Multisig]
		[ctype, Ctype]
		[attestation, Attestation]
		[delegation, Delegation]
		[did, Did]
		[pallet_inflation, Inflation]
		[pallet_did_lookup, DidLookup]
		[pallet_web3_names, Web3Names]
		[public_credentials, PublicCredentials]
		[pallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
		[pallet_migration, Migration]
		[pallet_dip_provider, DipProvider]
		[pallet_deposit_storage, DepositStorage]
		[pallet_asset_switch, AssetSwitchPool1]
		[pallet_assets, Fungibles]
		[pallet_message_queue, MessageQueue]
		[cumulus_pallet_parachain_system, ParachainSystem]
		[frame_benchmarking::baseline, Baseline::<Runtime>]
	);

	// Required since the pallet `AssetTransactor` will try to deduct the XCM fee
	// from the user's balance, and the asset must exist.
	pub struct CreateFungibleForAssetSwitchPool1;

	impl pallet_asset_switch::BenchmarkHelper for CreateFungibleForAssetSwitchPool1 {
		fn setup() -> Option<PartialBenchmarkInfo> {
			const DESTINATION_PARA_ID: u32 = 1_000;

			let asset_location: Location = Junctions::Here.into();
			Fungibles::create(
				RawOrigin::Root.into(),
				asset_location.clone(),
				AccountId::from([0; 32]).into(),
				1u32.into(),
			)
			.unwrap();
			let beneficiary = Junctions::X1(
				[Junction::AccountId32 {
					network: None,
					id: [0; 32],
				}]
				.into(),
			)
			.into();
			let destination = Location::from(ParentThen(Junctions::X1(
				[Junction::Parachain(DESTINATION_PARA_ID)].into(),
			)))
			.into();
			let remote_xcm_fee = Asset {
				id: AssetId(asset_location),
				fun: Fungibility::Fungible(1_000),
			}
			.into();

			ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(DESTINATION_PARA_ID.into());

			Some(PartialBenchmarkInfo {
				beneficiary: Some(beneficiary),
				destination: Some(destination),
				remote_asset_id: None,
				remote_xcm_fee: Some(remote_xcm_fee),
			})
		}
	}
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			frame_system::Pallet::<Runtime>::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}

		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}

		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
	for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(
			extrinsic: <Block as BlockT>::Extrinsic,
		) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: sp_inherents::InherentData) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}

		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuthorityId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
		}

		fn authorities() -> Vec<AuthorityId> {
			Aura::authorities().into_inner()
		}
	}

	impl cumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
		fn can_build_upon(
			included_hash: <Block as BlockT>::Hash,
			slot: cumulus_primitives_aura::Slot,
			) -> bool {
				ConsensusHook::can_build_upon(included_hash, slot)
			}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}

	impl kilt_runtime_api_did::Did<
		Block,
		DidIdentifier,
		AccountId,
		LinkableAccountId,
		Balance,
		Hash,
		BlockNumber
	> for Runtime {
		fn query_by_web3_name(name: Vec<u8>) -> Option<kilt_runtime_api_did::RawDidLinkedInfo<
				DidIdentifier,
				AccountId,
				LinkableAccountId,
				Balance,
				Hash,
				BlockNumber
			>
		> {
			let name: pallet_web3_names::web3_name::AsciiWeb3Name<Runtime> = name.try_into().ok()?;
			pallet_web3_names::Owner::<Runtime>::get(&name)
				.and_then(|owner_info| {
					did::Did::<Runtime>::get(&owner_info.owner).map(|details| (owner_info, details))
				})
				.map(|(owner_info, details)| {
					let accounts = pallet_did_lookup::ConnectedAccounts::<Runtime>::iter_key_prefix(
						&owner_info.owner,
					).collect();
					let service_endpoints = did::ServiceEndpoints::<Runtime>::iter_prefix(&owner_info.owner).map(|e| From::from(e.1)).collect();

					kilt_runtime_api_did::RawDidLinkedInfo{
						identifier: owner_info.owner,
						w3n: Some(name.into()),
						accounts,
						service_endpoints,
						details: details.into(),
					}
			})
		}

		fn query_by_account(account: LinkableAccountId) -> Option<
			kilt_runtime_api_did::RawDidLinkedInfo<
				DidIdentifier,
				AccountId,
				LinkableAccountId,
				Balance,
				Hash,
				BlockNumber
			>
		> {
			pallet_did_lookup::ConnectedDids::<Runtime>::get(account)
				.and_then(|owner_info| {
					did::Did::<Runtime>::get(&owner_info.did).map(|details| (owner_info, details))
				})
				.map(|(connection_record, details)| {
					let w3n = pallet_web3_names::Names::<Runtime>::get(&connection_record.did).map(Into::into);
					let accounts = pallet_did_lookup::ConnectedAccounts::<Runtime>::iter_key_prefix(&connection_record.did).collect();
					let service_endpoints = did::ServiceEndpoints::<Runtime>::iter_prefix(&connection_record.did).map(|e| From::from(e.1)).collect();

					kilt_runtime_api_did::RawDidLinkedInfo {
						identifier: connection_record.did,
						w3n,
						accounts,
						service_endpoints,
						details: details.into(),
					}
				})
		}

		fn query(did: DidIdentifier) -> Option<
			kilt_runtime_api_did::RawDidLinkedInfo<
				DidIdentifier,
				AccountId,
				LinkableAccountId,
				Balance,
				Hash,
				BlockNumber
			>
		> {
			let details = did::Did::<Runtime>::get(&did)?;
			let w3n = pallet_web3_names::Names::<Runtime>::get(&did).map(Into::into);
			let accounts = pallet_did_lookup::ConnectedAccounts::<Runtime>::iter_key_prefix(&did).collect();
			let service_endpoints = did::ServiceEndpoints::<Runtime>::iter_prefix(&did).map(|e| From::from(e.1)).collect();

			Some(kilt_runtime_api_did::RawDidLinkedInfo {
				identifier: did,
				w3n,
				accounts,
				service_endpoints,
				details: details.into(),
			})
		}
	}

	impl kilt_runtime_api_public_credentials::PublicCredentials<Block, Vec<u8>, Hash, public_credentials::CredentialEntry<Hash, DidIdentifier, BlockNumber, AccountId, Balance, AuthorizationId<<Runtime as delegation::Config>::DelegationNodeId>>, PublicCredentialsFilter<Hash, AccountId>, PublicCredentialsApiError> for Runtime {
		fn get_by_id(credential_id: Hash) -> Option<public_credentials::CredentialEntry<Hash, DidIdentifier, BlockNumber, AccountId, Balance, AuthorizationId<<Runtime as delegation::Config>::DelegationNodeId>>> {
			let subject = public_credentials::CredentialSubjects::<Runtime>::get(credential_id)?;
			public_credentials::Credentials::<Runtime>::get(subject, credential_id)
		}

		fn get_by_subject(subject: Vec<u8>, filter: Option<PublicCredentialsFilter<Hash, AccountId>>) -> Result<Vec<(Hash, public_credentials::CredentialEntry<Hash, DidIdentifier, BlockNumber, AccountId, Balance, AuthorizationId<<Runtime as delegation::Config>::DelegationNodeId>>)>, PublicCredentialsApiError> {
			let asset_did = AssetDid::try_from(subject).map_err(|_| PublicCredentialsApiError::InvalidSubjectId)?;
			let credentials_prefix = public_credentials::Credentials::<Runtime>::iter_prefix(asset_did);
			if let Some(filter) = filter {
				Ok(credentials_prefix.filter(|(_, entry)| filter.should_include(entry)).collect())
			} else {
				Ok(credentials_prefix.collect())
			}
		}
	}

	impl kilt_runtime_api_staking::Staking<Block, AccountId, Balance> for Runtime {
		fn get_unclaimed_staking_rewards(account: &AccountId) -> Balance {
			ParachainStaking::get_unclaimed_staking_rewards(account)
		}

		fn get_staking_rates() -> kilt_runtime_api_staking::StakingRates {
			ParachainStaking::get_staking_rates()
		}
	}

	impl kilt_runtime_api_dip_provider::DipProvider<Block, dip::runtime_api::DipProofRequest, CompleteMerkleProof<Hash, DidMerkleProofOf<Runtime>>, dip::runtime_api::DipProofError> for Runtime {
		fn generate_proof(request: dip::runtime_api::DipProofRequest) -> Result<CompleteMerkleProof<Hash, DidMerkleProofOf<Runtime>>, dip::runtime_api::DipProofError> {
			use pallet_dip_provider::traits::IdentityProvider;

			let identity_details = pallet_dip_provider::IdentityProviderOf::<Runtime>::retrieve(&request.identifier).map_err(dip::runtime_api::DipProofError::IdentityProvider)?;
			log::info!(target: "runtime_api::dip_provider", "Identity details retrieved for request {:#?}: {:#?}", request, identity_details);

			DidMerkleRootGenerator::<Runtime>::generate_proof(&identity_details, request.version, request.keys.iter(), request.should_include_web3_name, request.accounts.iter()).map_err(dip::runtime_api::DipProofError::MerkleProof)
		}
	}

	impl pallet_asset_switch_runtime_api::AssetSwitch<Block, VersionedAssetId, AccountId, AssetSwitchApiError> for Runtime {
		fn pool_account_id(pair_id: Vec<u8>, asset_id: VersionedAssetId) -> Result<AccountId, AssetSwitchApiError> {
			use core::str;
			use frame_support::traits::PalletInfoAccess;

			let Ok(pair_id_as_string) = str::from_utf8(pair_id.as_slice()) else {
				return Err(AssetSwitchApiError::InvalidInput);
			};
			match pair_id_as_string {
				kilt_to_ekilt if kilt_to_ekilt == AssetSwitchPool1::name() => {
					AssetSwitchPool1::pool_account_id_for_remote_asset(&asset_id).map_err(|e| {
						log::error!("Failed to calculate pool account address for asset ID {:?} with error: {:?}", asset_id, e);
						AssetSwitchApiError::Internal
					})
				},
				_ => Err(AssetSwitchApiError::SwitchPoolNotFound)
			}
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;

			use frame_system_benchmarking::Pallet as SystemBench;
			use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
			use frame_benchmarking::baseline::Pallet as Baseline;
			use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsicsBenchmark;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, BenchmarkError};
			use frame_system_benchmarking::Pallet as SystemBench;
			use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
			use frame_benchmarking::baseline::Pallet as Baseline;
			use frame_support::traits::TrackedStorageKey;
			use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsicsBenchmark;
			use runtime_common::benchmarks::xcm as xcm_benchmarking;
			use xcm::lts::prelude::*;

			impl pallet_xcm::benchmarking::Config for Runtime {
				type DeliveryHelper = xcm_benchmarking::ParachainDeliveryHelper<ParachainSystem, xcm_config::XcmConfig>;

				fn reachable_dest() -> Option<Location> {
					ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(xcm_benchmarking::RandomParaId::get());
					Some(xcm_benchmarking::ParachainLocation::get())
				}

				fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
					ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(xcm_benchmarking::RandomParaId::get());
					Some((
						xcm_benchmarking::NativeAsset::get(),
						xcm_benchmarking::ParachainLocation::get(),
					))
				}

				fn set_up_complex_asset_transfer() -> Option<(Assets, u32, Location, Box<dyn FnOnce()>)> {
					let (transferable_asset, dest) = Self::reserve_transferable_asset_and_dest().unwrap();

					let fee_amount = ExistentialDeposit::get();
					let fee_asset: Asset = (Location::here(), fee_amount).into();

					// Make account free to pay the fee
					let who = frame_benchmarking::whitelisted_caller();
					let balance = fee_amount + ExistentialDeposit::get() * 1000;
					let _ = <Balances as frame_support::traits::Currency<_>>::make_free_balance_be(
						&who, balance,
					);

					// verify initial balance
					assert_eq!(Balances::free_balance(&who), balance);


					let assets: Assets = vec![fee_asset.clone(), transferable_asset.clone()].into();
					let fee_index = if assets.get(0).unwrap().eq(&fee_asset) { 0 } else { 1 };

					let verify = Box::new( move || {
						let Fungibility::Fungible(transferable_amount) = transferable_asset.fun else { return; };
						assert_eq!(Balances::free_balance(&who), balance - fee_amount - transferable_amount);
					});

					Some((assets,fee_index , dest, verify))
				}

				fn get_asset() -> Asset {
					xcm_benchmarking::NativeAsset::get()
				}
			}

			impl frame_system_benchmarking::Config for Runtime {
				   fn setup_set_code_requirements(code: &sp_std::vec::Vec<u8>) -> Result<(), BenchmarkError> {
					   ParachainSystem::initialize_for_set_code_benchmark(code.len() as u32);
					   Ok(())
				   }

				fn verify_set_code() {
					System::assert_last_event(cumulus_pallet_parachain_system::Event::<Runtime>::ValidationFunctionStored.into());
				}
			}

			impl cumulus_pallet_session_benchmarking::Config for Runtime {}
			impl frame_benchmarking::baseline::Config for Runtime {}

			let whitelist: Vec<TrackedStorageKey> = vec![
				// Block Number
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac")
					.to_vec().into(),
				// Total Issuance
				hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80")
					.to_vec().into(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a")
					.to_vec().into(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850")
					.to_vec().into(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7")
					.to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmarks!(params, batches);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: UpgradeCheckSelect) -> (Weight, Weight) {
			log::info!("try-runtime::on_runtime_upgrade peregrine.");
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, BlockWeights::get().max_block)
		}

		fn execute_block(block: Block, state_root_check: bool, sig_check: bool, select: frame_try_runtime::TryStateSelect) -> Weight {
			log::info!(
				target: "runtime::peregrine", "try-runtime: executing block #{} ({:?}) / root checks: {:?} / sig check: {:?} / sanity-checks: {:?}",
				block.header.number,
				block.header.hash(),
				state_root_check,
				sig_check,
				select,
			);
			Executive::try_execute_block(block, state_root_check, sig_check, select).expect("try_execute_block failed")
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {

		fn create_default_config() -> Vec<u8> {
			create_default_config::<RuntimeGenesisConfig>()
		}

		fn build_config(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_config::<RuntimeGenesisConfig>(config)
		}

	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
