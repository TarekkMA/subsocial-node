use sp_core::H256;
use sp_io::TestExternalities;

use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup, Zero},
    testing::Header,
    Storage,
};

use frame_support::{
    assert_ok, assert_noop,
    parameter_types,
    dispatch::{DispatchResult, DispatchError},
    traits::Everything,
};

use pallet_permissions::{
    SpacePermission,
    SpacePermission as SP,
    SpacePermissions,
};
use pallet_posts::{Post, PostUpdate, PostExtension, Comment, Error as PostsError};
use pallet_profiles::{ProfileUpdate, Error as ProfilesError};
use pallet_profile_follows::Error as ProfileFollowsError;
use pallet_reactions::{ReactionId, ReactionKind, Error as ReactionsError};
use pallet_spaces::{SpaceById, SpaceUpdate, Error as SpacesError, SpacesSettings};
use pallet_space_follows::Error as SpaceFollowsError;
use pallet_space_ownership::Error as SpaceOwnershipError;
use pallet_moderation::{EntityId, EntityStatus, ReportId};
use pallet_permissions::default_permissions::DefaultSpacePermissions;
use pallet_utils::{
    mock_functions::*,
    SpaceId, PostId, User, Content,
};

use crate::mock::*;

/* Integration tests mocks */

pub(crate) const ACCOUNT1: AccountId = 1;
pub(crate) const ACCOUNT2: AccountId = 2;
pub(crate) const ACCOUNT3: AccountId = 3;

pub(crate) const SPACE1: SpaceId = 1001;
pub(crate) const SPACE2: SpaceId = 1002;

pub(crate) const POST1: PostId = 1;
pub(crate) const POST2: PostId = 2;
pub(crate) const POST3: PostId = 3;

pub(crate) const REACTION1: ReactionId = 1;
pub(crate) const REACTION2: ReactionId = 2;