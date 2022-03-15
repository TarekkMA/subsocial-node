//! All related code to free-calls module

use frame_support::log::{debug, info};
use sp_std::convert::TryInto;
use frame_support::traits::Contains;
use sp_std::cmp::min;
use sp_std::if_std;
use static_assertions::const_assert;
use pallet_free_calls::{NumberOfCalls, QuotaToWindowRatio, WindowConfig};
use pallet_locker_mirror::LockedInfoOf;
use crate::BlockNumber;
use super::constants::time::*;
use super::constants::currency;
use super::{Runtime, Call};

// TODO: try to find a better way to calculate it based on the circulating supply
pub const FREE_CALLS_PER_SUB: u16 = 10;

pub const FREE_CALLS_WINDOWS_CONFIG: [WindowConfig<BlockNumber>; 3] = [
    // Window that lasts a day and has 100% of the allocated quota.
    WindowConfig::new(1 * DAYS, QuotaToWindowRatio::new(1)),
    // Window that lasts an hour and has (1/3) of the allocated quota.
    WindowConfig::new(1 * HOURS, QuotaToWindowRatio::new(3)),
    // Window that lasts for 5 minutes and has (1/10) of the allocated quota.
    WindowConfig::new(5 * MINUTES, QuotaToWindowRatio::new(10)),
];


// Assert at compile time that the free-calls configs are in the optimal shape.
const_assert!(check_free_calls_config(&FREE_CALLS_WINDOWS_CONFIG));
#[allow(dead_code)] // the code is not acutely dead.
const fn check_free_calls_config(configs: &'static [WindowConfig<BlockNumber>]) -> bool {
    // cannot have empty configs
    if configs.is_empty() {
        return false;
    }
    let mut config = &configs[0];
    // first config must have 1 as ratio
    if config.quota_ratio.get() != 1 {
        return false;
    }

    let mut i = 1;

    while i < configs.len() {
        let current_config = &configs[i];

        // current period must be less than the previous period
        if current_config.period >= config.period {
            return false;
        }

        // current ratio must be grater than or equal the previous ratio
        if current_config.quota_ratio.get() < config.quota_ratio.get() {
            return false;
        }

        config = current_config;
        i = i + 1;
    }

    return true;
}

/// Filter the calls that can be used as free calls.
// TODO: add more calls to this filter. or maybe allow all calls???
pub struct FreeCallsFilter;
impl Default for FreeCallsFilter { fn default() -> Self { Self } }
impl Contains<Call> for FreeCallsFilter {
    fn contains(c: &Call) -> bool {
        match *c {
            Call::Spaces(..) => true,
            Call::SpaceFollows(..) => true,
            Call::ProfileFollows(..) => true,
            Call::Posts(..) => true,
            Call::Reactions(..) => true,
            Call::System(..) => cfg!(feature = "runtime-benchmarks"),
            _ => false,
        }
    }
}

/// A calculation strategy for free calls quota
pub struct FreeCallsCalculationStrategy;
impl Default for FreeCallsCalculationStrategy { fn default() -> Self { Self } }
impl pallet_free_calls::QuotaCalculationStrategy<Runtime> for FreeCallsCalculationStrategy {
    fn calculate(
        consumer: <Runtime as frame_system::Config>::AccountId,
        current_block: <Runtime as frame_system::Config>::BlockNumber,
        locked_info: Option<LockedInfoOf<Runtime>>
    ) -> Option<NumberOfCalls> {
        fn get_utilization_percent(lock_period: BlockNumber) -> u64 {
            if lock_period < 1 * WEEKS {
                return 15;
            }
            if lock_period < 1 * MONTHS {
                let num_of_weeks = min(3, lock_period / (1 * WEEKS)) as u64;
                return (num_of_weeks * 5) + 25;
            }

            let num_of_months = min(12, lock_period / (1 * MONTHS)) as u64;
            return (num_of_months * 5) + 40;
        }

        let LockedInfoOf::<Runtime>{
            locked_at,
            locked_amount,
            expires_at,
        } = match locked_info {
            Some(locked_info) => locked_info,
            None => return None,
        };

        if locked_at >= current_block {
            return None;
        }

        if matches!(expires_at, Some(expires_at) if current_block >= expires_at) {
            return None;
        }

        let lock_period = current_block - locked_at;

        let utilization_percent = get_utilization_percent(lock_period);

        let num_of_tokens = locked_amount.saturating_div(currency::DOLLARS) as u64;

        let num_of_free_calls = num_of_tokens
            .saturating_mul(FREE_CALLS_PER_SUB.into())
            .saturating_mul(utilization_percent)
            .saturating_div(100);

        Some(num_of_free_calls.try_into().unwrap_or(NumberOfCalls::MAX))
    }
}


#[cfg(test)]
mod tests {
    use frame_benchmarking::account;
    use pallet_locker_mirror::LockedInfoOf;
    use pallet_free_calls::{NumberOfCalls, QuotaCalculationStrategy};
    use crate::*;
    use rstest::rstest;

    #[rstest]
    // FREE_CALLS_PER_SUB = 10
    #[case(1 * CENTS, 10, Some(0))]

    #[case(1 * DOLLARS, 1 * DAYS, Some(1))]
    #[case(10 * DOLLARS, 1 * DAYS, Some(15))]
    #[case(100 * DOLLARS, 1 * DAYS, Some(150))]

    #[case(1 * DOLLARS, 1 * WEEKS, Some(3))]
    #[case(10 * DOLLARS, 1 * WEEKS, Some(30))]

    #[case(1 * DOLLARS, 2 * WEEKS, Some(3))]
    #[case(10 * DOLLARS, 2 * WEEKS, Some(35))]

    #[case(1 * DOLLARS, 3 * WEEKS, Some(4))]
    #[case(10 * DOLLARS, 3 * WEEKS, Some(40))]

    // 4 weeks (28) is treated as 3 weeks
    #[case(1 * DOLLARS, 4 * WEEKS, Some(4))]
    #[case(10 * DOLLARS, 4 * WEEKS, Some(40))]

    #[case(5 * DOLLARS, 1 * MONTHS, Some(22))]
    #[case(20 * DOLLARS, 1 * MONTHS, Some(90))]

    #[case(5 * DOLLARS, 2 * MONTHS, Some(25))]
    #[case(20 * DOLLARS, 2 * MONTHS, Some(100))]

    #[case(5 * DOLLARS, 3 * MONTHS, Some(27))]
    #[case(20 * DOLLARS, 3 * MONTHS, Some(110))]

    #[case(5 * DOLLARS, 4 * MONTHS, Some(30))]
    #[case(20 * DOLLARS, 4 * MONTHS, Some(120))]

    #[case(5 * DOLLARS, 5 * MONTHS, Some(32))]
    #[case(20 * DOLLARS, 5 * MONTHS, Some(130))]
    #[case(500 * DOLLARS, 5 * MONTHS, Some(3250))]

    // treated as 5 MONTHS
    #[case(500 * DOLLARS, 5 * MONTHS + 1 * WEEKS, Some(3250))]

    #[case(100 * DOLLARS, 6 * MONTHS, Some(700))]
    #[case(100 * DOLLARS, 7 * MONTHS, Some(750))]
    #[case(100 * DOLLARS, 8 * MONTHS, Some(800))]
    #[case(100 * DOLLARS, 9 * MONTHS, Some(850))]
    #[case(100 * DOLLARS, 10 * MONTHS, Some(900))]
    #[case(100 * DOLLARS, 11 * MONTHS, Some(950))]
    #[case(100 * DOLLARS, 12 * MONTHS, Some(1000))]

    #[case(100 * DOLLARS, 13 * MONTHS, Some(1000))]
    #[case(100 * DOLLARS, 100 * MONTHS, Some(1000))]
    #[trace]
    fn quota_calculation_strategy_tests(
        #[case] amount: Balance,
        #[case] locked_period: BlockNumber,
        #[case] expected_quota: Option<NumberOfCalls>,
    ) {
        let current_block = 1000 * MONTHS;
        let before_current_block = current_block - 1;
        let after_current_block = current_block + 1;


        let locked_at = current_block - locked_period;
        let locked_info = LockedInfoOf::<Runtime> {
            locked_at,
            locked_amount: amount.into(),
            expires_at: None,
        };

        let locked_info_not_yet_expired = {
            let mut locked_info = locked_info.clone();
            locked_info.expires_at = Some(after_current_block);
            locked_info
        };

        let locked_info_expired = {
            let mut locked_info = locked_info.clone();
            locked_info.expires_at = Some(before_current_block);
            locked_info
        };

        let locked_info_just_expired = {
            let mut locked_info = locked_info.clone();
            locked_info.expires_at = Some(current_block);
            locked_info
        };

        ///////////////////////////////////////
        let consumer = || account("Dummy Consumer", 0, 0);

        // no locked_info will returns none
        assert_eq!(
            FreeCallsCalculationStrategy::calculate(consumer(), current_block, None),
            None,
        );
        assert_eq!(
            FreeCallsCalculationStrategy::calculate(consumer(),before_current_block, None),
            None,
        );
        assert_eq!(
            FreeCallsCalculationStrategy::calculate(consumer(),after_current_block, None),
            None,
        );

        assert_eq!(
            FreeCallsCalculationStrategy::calculate(consumer(),current_block, Some(locked_info)),
            expected_quota,
        );

        // test expiration
        assert_eq!(
            FreeCallsCalculationStrategy::calculate(consumer(),current_block, Some(locked_info_just_expired)),
            None,
        );
        assert_eq!(
            FreeCallsCalculationStrategy::calculate(consumer(),current_block, Some(locked_info_expired)),
            None,
        );
        assert_eq!(
            FreeCallsCalculationStrategy::calculate(consumer(),current_block, Some(locked_info_not_yet_expired)),
            expected_quota,
        );

    }
}