use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup}, testing::Header, Storage
};

use crate as pallet_free_calls;

use frame_support::{
    parameter_types,
    assert_ok,
    dispatch::DispatchResultWithPostInfo,
};
use frame_support::traits::Everything;
use frame_system as system;
use frame_system::EnsureRoot;
use pallet_locker_mirror::{BalanceOf, LockedInfo, LockedInfoOf};

pub(crate) type AccountId = u64;
pub(crate) type BlockNumber = u64;

use crate::mock::time::*;
use crate::{NumberOfCalls, QuotaToWindowRatio, WindowConfig};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub mod time {
    use crate::mock::BlockNumber;

    pub const MILLISECS_PER_BLOCK: BlockNumber = 6000;
    pub const SLOT_DURATION: BlockNumber = MILLISECS_PER_BLOCK;

    // These time units are defined in number of blocks.
    pub const MINUTES: BlockNumber = 60_000 / MILLISECS_PER_BLOCK;
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;
}

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: system::{Pallet, Call, Config, Storage, Event<T>},
        FreeCalls: pallet_free_calls::{Pallet, Call, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        LockerMirror: pallet_locker_mirror::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 28;
}

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type Balance = u64;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = ();
}


impl pallet_locker_mirror::Config for Test {
    type Event = Event;
    type Currency = Balances;
    type OracleOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
}

/// A calculation strategy for free calls quota
pub struct FreeCallsCalculationStrategy;
impl Default for FreeCallsCalculationStrategy { fn default() -> Self { Self } }
impl pallet_free_calls::QuotaCalculationStrategy<Test> for FreeCallsCalculationStrategy {
    fn calculate(
        current_block: <Test as frame_system::Config>::BlockNumber,
        locked_info: Option<LockedInfoOf<Test>>
    ) -> Option<NumberOfCalls> {
        locked_info.and_then(|locked_info| {
            if current_block >= locked_info.unlocks_at {
                None
            } else {
                // TODO: add more sophisticated calculation
                // TODO: think if we should make NumberOfCalls -> u32 instead of u16
                Some((locked_info.locked_amount / 11 /*decimals*/) as NumberOfCalls)
            }
        })
    }
}

parameter_types! {
    pub static WindowsConfig: Vec<WindowConfig<BlockNumber>> = [
        WindowConfig::new(1 * DAYS, QuotaToWindowRatio::new(1)),
        WindowConfig::new(2 * HOURS, QuotaToWindowRatio::new(3)),
        WindowConfig::new(30 * MINUTES, QuotaToWindowRatio::new(5)),
        WindowConfig::new(5 * MINUTES, QuotaToWindowRatio::new(20)),
        WindowConfig::new(1, QuotaToWindowRatio::new(1000)),
    ].to_vec();
}

impl pallet_free_calls::Config for Test {
    type Event = Event;
    type Call = Call;
    type WindowsConfig = WindowsConfig;
    type CallFilter = Everything;
    type WeightInfo = ();
    type QuotaCalculationStrategy = FreeCallsCalculationStrategy;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> TestExternalities {
    frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
