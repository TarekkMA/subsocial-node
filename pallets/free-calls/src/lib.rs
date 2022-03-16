//! # Free Calls Pallet
//!
//! Pallet for allowing accounts to send free calls based on a set quota.
//! The quota can be distributed over multiple overlapping windows to limit abuse.
//!
//! Resources:
//! - https://cloud.google.com/architecture/rate-limiting-strategies-techniques
//! - https://www.figma.com/blog/an-alternative-approach-to-rate-limiting/
//! - https://www.codementor.io/@arpitbhayani/system-design-sliding-window-based-rate-limiter-157x7sburi
//! - https://blog.cloudflare.com/counting-things-a-lot-of-different-things/

#![cfg_attr(not(feature = "std"), no_std)]
// #![feature(const_panic)] not needed for the new rust version

use codec::{Decode, Encode};
use frame_support::ensure;
use frame_support::traits::IsSubType;
use sp_runtime::traits::DispatchInfoOf;
use sp_runtime::traits::SignedExtension;
use sp_runtime::transaction_validity::InvalidTransaction;
use sp_runtime::transaction_validity::TransactionValidity;
use sp_runtime::transaction_validity::TransactionValidityError;
use sp_runtime::transaction_validity::ValidTransaction;
use sp_std::fmt::Debug;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test_pallet;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod weights;

pub use weights::WeightInfo;
use frame_support::traits::Contains;
use scale_info::TypeInfo;

#[frame_support::pallet]
pub mod pallet {
    use sp_std::convert::TryInto;
    use sp_std::num::NonZeroU16;
    use frame_support::weights::{extract_actual_weight, GetDispatchInfo};
    use frame_support::{dispatch::DispatchResult, log, pallet_prelude::*};
    use frame_support::dispatch::PostDispatchInfo;
    use sp_std::default::Default;
    use frame_support::traits::{Contains, IsSubType};
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{Dispatchable};
    use sp_runtime::traits::Zero;
    use sp_std::boxed::Box;
    use sp_std::cmp::max;
    use sp_std::vec::Vec;
    use pallet_locker_mirror::{LockedInfoByAccount, LockedInfoOf};
    use pallet_utils::bool_to_option;
    use scale_info::TypeInfo;
    use crate::WeightInfo;

    /// The ratio between the quota and a particular window.
    ///
    /// ## Example:
    /// if ratio is 20 and the quota is 100 then each window should have maximum of 5 calls.
    /// max number of calls per window = quota / ratio.
    pub type QuotaToWindowRatio = NonZeroU16;

    /// Type to keep track of how many calls is in quota or used in a particular window.
    pub type NumberOfCalls = u16;

    /// A `BoundedVec` that can hold a list of `ConsumerStats` objects bounded by the size of WindowConfigs.
    pub type ConsumerStatsVec<T> = BoundedVec<ConsumerStats<<T as frame_system::Config>::BlockNumber>, WindowsConfigSize<T>>;

    /// Keeps track of the executed number of calls per window per consumer.
    #[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
    pub struct ConsumerStats<BlockNumber> {
        // TODO: find a better name? maybe `stats_index`
        /// The index of this window in the timeline.
        pub timeline_index: BlockNumber,

        /// The number of calls executed during this window.
        pub used_calls: NumberOfCalls,
    }

    impl<BlockNumber> ConsumerStats<BlockNumber> {
        fn new(window_index: BlockNumber) -> Self {
            ConsumerStats {
                timeline_index: window_index,
                used_calls: 0,
            }
        }
    }

    /// Configuration of a rate limiting window in terms of length and ratio to quota.
    #[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
    pub struct WindowConfig<BlockNumber> {
        /// The length of the window in number of blocks it will last.
        pub period: BlockNumber,

        /// The ratio between the quota and this window.
        pub quota_ratio: QuotaToWindowRatio,
    }

    impl<BlockNumber> WindowConfig<BlockNumber> {
        //TODO: try to also force period to be non zero.
        pub const fn new(period: BlockNumber, quota_ratio: Option<QuotaToWindowRatio>) -> Self {
            WindowConfig {
                period,
                quota_ratio: match quota_ratio {
                    Some(non_zero) => non_zero,
                    None => panic!("quota_ratio must be non zero"),
                },
            }
        }
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_locker_mirror::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The call type from the runtime which has all the calls available in your runtime.
        type Call: Parameter
            + Dispatchable<Origin = Self::Origin, PostInfo = PostDispatchInfo>
            + GetDispatchInfo
            + From<frame_system::Call<Self>>
            + IsSubType<Call<Self>>
            + IsType<<Self as frame_system::Config>::Call>;

        /// The configurations that will be used to limit the usage of the allocated quota to these
        /// different configs.
        #[pallet::constant]
        type WindowsConfig: Get<Vec<WindowConfig<Self::BlockNumber>>>;

        /// Filter on which calls are permitted to be free.
        type CallFilter: Contains<<Self as Config>::Call>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// A calculation strategy to convert locked tokens info to a quota.
        type QuotaCalculationStrategy: QuotaCalculationStrategy<Self>;

        /// Maximum number of accounts that can be added as eligible at a time.
        //TODO: remove this after we integrate locking tokens
        #[pallet::constant]
        type AccountsSetLimit: Get<u32>;

        /// Amount of free quota granted to eligible accounts.
        //TODO: remove this after we integrate locking tokens
        #[pallet::constant]
        type FreeQuotaPerEligibleAccount: Get<NumberOfCalls>;
    }

    /// Retrieves the size of `T::WindowsConfig` to be used for `BoundedVec` declaration.
    pub struct WindowsConfigSize<T: Config>(PhantomData<T>);

    impl<T: Config> Default for WindowsConfigSize<T> {
        fn default() -> Self {
            Self(PhantomData)
        }
    }

    impl<T: Config> Get<u32> for WindowsConfigSize<T> {
        fn get() -> u32 {
            T::WindowsConfig::get().len().try_into().unwrap()
        }
    }

    /// Keeps track of each windows usage for each consumer.
    #[pallet::storage]
    #[pallet::getter(fn window_stats_by_consumer)]
    pub(super) type WindowStatsByConsumer<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        ConsumerStatsVec<T>,
        ValueQuery,
    >;

    /// Keeps track of all eligible accounts for free calls
    //TODO: remove this after we integrate locking tokens
    #[pallet::storage]
    pub(super) type EligibleAccounts<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        bool,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Free call was executed. [who, result]
        FreeCallResult(T::AccountId, DispatchResult),

        /// List of eligible accounts added. [number of added accounts]
        //TODO: remove this after we integrate locking tokens
        EligibleAccountsAdded(u16),
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {

        /// Try to execute a call using the free allocated quota. This call may not execute because
        /// one of the following reasons:
        ///  * Caller has no free quota set.
        ///  * The caller has used all the allowed quota for at least one window config.
        ///
        /// Pre-validation:
        /// This call is pre validated using `FreeCallsPrevalidation` signed extension and will only
        /// be valid if the consumer can have a free call.
        #[pallet::weight({
            let boxed_call_info = call.get_dispatch_info();
            let boxed_call_weight = boxed_call_info.weight;
            let self_weight = <T as Config>::WeightInfo::try_free_call();

            let total_weight = self_weight.saturating_add(boxed_call_weight);
            (
                total_weight,
                boxed_call_info.class,
                Pays::No,
            )
        })]
        pub fn try_free_call(
            origin: OriginFor<T>,
            call: Box<<T as Config>::Call>,
        ) -> DispatchResultWithPostInfo {
            let consumer = ensure_signed(origin.clone())?;

            let mut actual_weight = <T as Config>::WeightInfo::try_free_call();

            let maybe_new_stats = bool_to_option(T::CallFilter::contains(&call))
                .and_then(|_| Self::can_make_free_call(&consumer));

            if let Some(new_stats) = maybe_new_stats {

                Self::update_consumer_stats(consumer.clone(), new_stats);

                let info = call.get_dispatch_info();

                // Dispatch the call
                let result = call.dispatch(origin);

                // Add the current weight for the boxed call
                actual_weight = actual_weight.saturating_add(extract_actual_weight(&result, &info));

                // Deposit an event with the result
                Self::deposit_event(Event::FreeCallResult(
                    consumer,
                    result.map(|_| ()).map_err(|e| e.error),
                ));
            }

            Ok(PostDispatchInfo {
                actual_weight: Some(actual_weight),
                pays_fee: Pays::No,
            })
        }

        #[pallet::weight(
            <T as Config>::WeightInfo::add_eligible_accounts(
                eligible_accounts.len() as u32
            )
        )]
        pub fn add_eligible_accounts(
            origin: OriginFor<T>,
            eligible_accounts: BoundedVec<T::AccountId, T::AccountsSetLimit>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            let accounts_len = eligible_accounts.len();

            for eligible_account in eligible_accounts {
                <EligibleAccounts<T>>::insert(&eligible_account, true);
            }

            Self::deposit_event(Event::EligibleAccountsAdded(accounts_len as u16));
            Ok(Pays::No.into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Determine if `consumer` can have a free call.
        ///
        /// If the consumer can have a free call the new stats that should be applied will be returned,
        /// otherwise None is returned.
        pub fn can_make_free_call(consumer: &T::AccountId) -> Option<ConsumerStatsVec<T>> {
            let current_block = <frame_system::Pallet<T>>::block_number();

            let windows_config = T::WindowsConfig::get();

            if windows_config.is_empty() {
                return None;
            }

            let locked_info = <LockedInfoByAccount<T>>::get(consumer.clone());
            let quota = match T::QuotaCalculationStrategy::calculate(consumer.clone(), current_block, locked_info) {
                Some(quota) if quota > 0 => quota,
                _ => return None,
            };

            let old_stats: ConsumerStatsVec<T> = Self::window_stats_by_consumer(consumer.clone());
            let mut new_stats: ConsumerStatsVec<T> = Default::default();

            for (config_index, config) in windows_config.into_iter().enumerate() {
                let new_window_stats = Self::check_window(
                    current_block,
                    quota,
                    config,
                    old_stats.get(config_index),
                );

                match new_window_stats {
                    None => {
                        return None;
                    },
                    Some(window_stats) => {
                        if matches!(new_stats.try_push(window_stats), Err(_)) {
                            return None;
                        }
                    }
                };
            }

            return Some(new_stats);
        }

        /// Checks if a window can allow one more call given its config and the last stored stats for
        /// the consumer.
        ///
        /// If the window can allow one more call, the new stats object is returned, otherwise `None`
        /// is returned.
        fn check_window(
            current_block: T::BlockNumber,
            quota: NumberOfCalls,
            config: WindowConfig<T::BlockNumber>,
            window_stats: Option<&ConsumerStats<T::BlockNumber>>,
        ) -> Option<ConsumerStats<T::BlockNumber>> {

            if config.period.is_zero() {
                return None;
            }

            let timeline_index = current_block / config.period;

            let reset_stats = || ConsumerStats::new(timeline_index);

            let mut stats = window_stats
                .map(|r| r.clone())
                .unwrap_or_else(reset_stats);

            if stats.timeline_index < timeline_index {
                stats = reset_stats();
            }

            let can_be_called = stats.used_calls < max(1, quota / config.quota_ratio);

            can_be_called.then(|| {
                stats.used_calls = stats.used_calls.saturating_add(1);
                stats
            })
        }

        pub fn update_consumer_stats(consumer: T::AccountId, new_stats: ConsumerStatsVec<T>) {
            log::info!("{:?} updating consumer stats", consumer);
            <WindowStatsByConsumer<T>>::insert(
                consumer,
                new_stats,
            );
        }
    }


    pub trait QuotaCalculationStrategy<T: Config> {
        fn calculate(
            consumer: T::AccountId,
            current_block: T::BlockNumber,
            locked_info: Option<LockedInfoOf<T>>
        ) -> Option<NumberOfCalls>;
    }

    //TODO: remove this after we integrate locking tokens
    impl<T: Config> QuotaCalculationStrategy<T> for () {
        fn calculate(
            consumer: T::AccountId,
            _current_block: T::BlockNumber,
            _locked_info: Option<LockedInfoOf<T>>
        ) -> Option<NumberOfCalls> {
            if EligibleAccounts::<T>::get(consumer) {
                Some(T::FreeQuotaPerEligibleAccount::get())
            } else {
                None
            }
        }
    }
}

/// Validate `try_free_call` calls prior to execution. Needed to avoid a DoS attack since they are
/// otherwise free to be included into blockchain.
#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct FreeCallsPrevalidation<T: Config + Send + Sync>(sp_std::marker::PhantomData<T>)
    where
        <T as frame_system::Config>::Call: IsSubType<Call<T>>;

impl<T: Config + Send + Sync> Debug for FreeCallsPrevalidation<T>
    where
        <T as frame_system::Config>::Call: IsSubType<Call<T>>,
{
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(f, "FreeCallsPrevalidation")
    }

    #[cfg(not(feature = "std"))]
    fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        Ok(())
    }
}

impl<T: Config + Send + Sync> FreeCallsPrevalidation<T>
    where
        <T as frame_system::Config>::Call: IsSubType<Call<T>>,
{
    /// Create new `SignedExtension` to check runtime version.
    pub fn new() -> Self {
        Self(sp_std::marker::PhantomData)
    }
}

#[repr(u8)]
pub enum FreeCallsValidityError {
    /// The caller is out of quota.
    OutOfQuota = 0,

    /// The call cannot be free.
    CallCannotBeFree = 1,
}

impl From<FreeCallsValidityError> for u8 {
    fn from(err: FreeCallsValidityError) -> Self {
        err as u8
    }
}

impl<T: Config + Send + Sync> SignedExtension for FreeCallsPrevalidation<T>
    where
        <T as frame_system::Config>::Call: IsSubType<Call<T>>,
{
    const IDENTIFIER: &'static str = "FreeCallsPrevalidation";

    type AccountId = T::AccountId;
    type Call = <T as frame_system::Config>::Call;
    type AdditionalSigned = ();
    type Pre = ();


    fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
        Ok(())
    }

    fn validate(
        &self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> TransactionValidity {
        if let Some(local_call) = call.is_sub_type() {
            if let Call::try_free_call { call: boxed_call } = local_call {
                ensure!(T::CallFilter::contains(boxed_call), InvalidTransaction::Custom(FreeCallsValidityError::CallCannotBeFree.into()));
                ensure!(Pallet::<T>::can_make_free_call(who).is_some(), InvalidTransaction::Custom(FreeCallsValidityError::OutOfQuota.into()));
            }
        }
        Ok(ValidTransaction::default())
    }
}
