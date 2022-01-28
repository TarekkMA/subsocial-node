pub mod currency {
	use subsocial_primitives::Balance;

	pub const UNITS: Balance = 100_000_000_000;
	pub const DOLLARS: Balance = UNITS;            // 100_000_000_000
	pub const CENTS: Balance = DOLLARS / 100;      // 1_000_000_000
	pub const MILLICENTS: Balance = CENTS / 1_000; // 1_000_000

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * CENTS + (bytes as Balance) * 6 * CENTS
	}
}

pub mod time {
	use subsocial_primitives::{Moment, BlockNumber};

	pub const MILLISECS_PER_BLOCK: Moment = 6000;
	pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

	// These time units are defined in number of blocks.
	pub const BLOCK: BlockNumber = (MILLISECS_PER_BLOCK as BlockNumber);
	pub const MINUTES: BlockNumber = 60_000 / BLOCK;
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;
}

pub mod free_calls {
    use pallet_free_calls::WindowConfig;
    use crate::BlockNumber;
    use super::time::*;

	/// Make sure that every next period is equal or smaller and ratio is equal or bigger.
    pub const FREE_CALLS_WINDOWS_CONFIG: [WindowConfig<BlockNumber>; 3] = [
        /// Window that lasts a day and has 100% of the allocated quota.
		WindowConfig::new(1 * DAYS, 1),
        /// Window that lasts an hour and has (1/3) of the allocated quota.
        WindowConfig::new(1 * HOURS, 3),
        /// Window that lasts for 5 minutes and has (1/10) of the allocated quota.
        WindowConfig::new(5 * MINUTES, 10),
    ];
}
