use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::thread::sleep;
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
use frame_support::traits::{Contains, Everything};
use frame_system as system;
use frame_system::{EnsureRoot, EventRecord};
use rand::Rng;
use pallet_locker_mirror::{BalanceOf, LockedInfo, LockedInfoOf};

pub(crate) type AccountId = u64;
pub(crate) type BlockNumber = u64;

use crate::mock::time::*;
use crate::{NumberOfCalls, QuotaToWindowRatio, WindowConfig, WindowStatsByConsumer};
use crate::tests::TestUtils;

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

////// Free Call Dependencies


type CallFilterFn = fn(&Call) -> bool;
static DEFAULT_CALL_FILTER_FN: CallFilterFn = |_| true;

type QuotaCalculationFn<T> = fn(<T as frame_system::Config>::BlockNumber, Option<LockedInfoOf<T>>) -> Option<NumberOfCalls>;
static DEFAULT_QUOTA_CALCULATION_FN: QuotaCalculationFn<Test> = |current_block, locked_info| {
    return Some(10);
};


pub static DEFAULT_WINDOWS_CONFIG: [WindowConfig<BlockNumber>; 1] = [
    WindowConfig::new(10, QuotaToWindowRatio::new(1)),
];

parameter_types! {
    pub static WindowsConfig: Vec<WindowConfig<BlockNumber>> = DEFAULT_WINDOWS_CONFIG.to_vec();
}

thread_local! {
    pub static CALL_FILTER: RefCell<CallFilterFn> = RefCell::new(DEFAULT_CALL_FILTER_FN);
    pub static QUOTA_CALCULATION: RefCell<QuotaCalculationFn<Test>> = RefCell::new(DEFAULT_QUOTA_CALCULATION_FN);
}

pub struct TestCallFilter;
impl Contains<Call> for TestCallFilter {
    fn contains(call: &Call) -> bool {
        CALL_FILTER.with(|filter| filter.borrow()(call))
    }
}

pub struct TestQuotaCalculation;
impl pallet_free_calls::QuotaCalculationStrategy<Test> for TestQuotaCalculation {
    fn calculate(
        current_block: <Test as frame_system::Config>::BlockNumber,
        locked_info: Option<LockedInfoOf<Test>>
    ) -> Option<NumberOfCalls> {
        QUOTA_CALCULATION.with(|strategy| strategy.borrow()(current_block, locked_info))
    }
}

impl pallet_free_calls::Config for Test {
    type Event = Event;
    type Call = Call;
    type WindowsConfig = WindowsConfig;
    type CallFilter = TestCallFilter;
    type WeightInfo = ();
    type QuotaCalculationStrategy = TestQuotaCalculation;
}

pub struct ExtBuilder {
    call_filter: CallFilterFn,
    quota_calculation: QuotaCalculationFn<Test>,
    windows_config: Vec<WindowConfig<BlockNumber>>,
}
impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            call_filter: DEFAULT_CALL_FILTER_FN,
            quota_calculation: DEFAULT_QUOTA_CALCULATION_FN,
            windows_config: DEFAULT_WINDOWS_CONFIG.to_vec(),
        }
    }
}
impl ExtBuilder {
    pub fn call_filter(mut self, call_filter: CallFilterFn) -> Self {
        self.call_filter = call_filter;
        self
    }

    pub fn quota_calculation(mut self, quota_calculation: QuotaCalculationFn<Test>) -> Self {
        self.quota_calculation = quota_calculation;
        self
    }

    pub fn windows_config(mut self, windows_config: Vec<WindowConfig<BlockNumber>>) -> Self {
        self.windows_config = windows_config;
        self
    }

    pub fn set_configs(&self) {
        CALL_FILTER.with(|filter| *filter.borrow_mut() = self.call_filter);
        QUOTA_CALCULATION.with(|calc| *calc.borrow_mut() = self.quota_calculation);
        WINDOWS_CONFIG.with(|configs| *configs.borrow_mut() = self.windows_config.clone());
    }

    pub fn build(self) -> TestExternalities {
        self.set_configs();

        let storage = &mut system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let mut ext = TestExternalities::from(storage.clone());
        ext.execute_with(|| TestUtils::set_block_number(1));

        ext
    }
}