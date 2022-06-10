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


pub(crate) fn permissions_where_everyone_can_create_post() -> SpacePermissions {
    let mut default_permissions = DefaultSpacePermissions::get();
    default_permissions.everyone = default_permissions.everyone
        .map(|mut permissions| {
            permissions.insert(SP::CreatePosts);
            permissions
        });

    default_permissions
}

pub(crate) fn permissions_where_follower_can_create_post() -> SpacePermissions {
    let mut default_permissions = DefaultSpacePermissions::get();
    default_permissions.follower = Some(vec![SP::CreatePosts].into_iter().collect());

    default_permissions
}


pub(crate) fn post_content_ipfs() -> Content {
    Content::IPFS(b"bafyreidzue2dtxpj6n4x5mktrt7las5wz5diqma47zr25uau743dhe76we".to_vec())
}

pub(crate) fn updated_post_content() -> Content {
    Content::IPFS(b"bafyreifw4omlqpr3nqm32bueugbodkrdne7owlkxgg7ul2qkvgrnkt3g3u".to_vec())
}

pub(crate) fn post_update(
    space_id: Option<SpaceId>,
    content: Option<Content>,
    hidden: Option<bool>,
) -> PostUpdate {
    PostUpdate {
        space_id,
        content,
        hidden,
    }
}

pub(crate) fn comment_content_ipfs() -> Content {
    Content::IPFS(b"bafyreib6ceowavccze22h2x4yuwagsnym2c66gs55mzbupfn73kd6we7eu".to_vec())
}

pub(crate) fn reply_content_ipfs() -> Content {
    Content::IPFS(b"QmYA2fn8cMbVWo4v95RwcwJVyQsNtnEwHerfWR8UNtEwoE".to_vec())
}

pub(crate) fn profile_content_ipfs() -> Content {
    Content::IPFS(b"QmRAQB6YaCyidP37UdDnjFY5vQuiaRtqdyoW2CuDgwxkA5".to_vec())
}

pub(crate) fn reaction_upvote() -> ReactionKind {
    ReactionKind::Upvote
}

pub(crate) fn reaction_downvote() -> ReactionKind {
    ReactionKind::Downvote
}

pub(crate) fn extension_regular_post() -> PostExtension {
    PostExtension::RegularPost
}

pub(crate) fn extension_comment(parent_id: Option<PostId>, root_post_id: PostId) -> PostExtension {
    PostExtension::Comment(Comment { parent_id, root_post_id })
}

pub(crate) fn extension_shared_post(post_id: PostId) -> PostExtension {
    PostExtension::SharedPost(post_id)
}

/// Account 2 follows Space 1
pub(crate) fn _default_follow_space() -> DispatchResult {
    _follow_space(None, None)
}

pub(crate) fn _follow_space(origin: Option<Origin>, space_id: Option<SpaceId>) -> DispatchResult {
    SpaceFollows::follow_space(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT2)),
        space_id.unwrap_or(SPACE1),
    )
}

pub(crate) fn _default_unfollow_space() -> DispatchResult {
    _unfollow_space(None, None)
}

pub(crate) fn _unfollow_space(origin: Option<Origin>, space_id: Option<SpaceId>) -> DispatchResult {
    SpaceFollows::unfollow_space(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT2)),
        space_id.unwrap_or(SPACE1),
    )
}

pub(crate) fn _create_default_post() -> DispatchResult {
    _create_post(None, None, None, None)
}

pub(crate) fn _create_post(
    origin: Option<Origin>,
    space_id_opt: Option<Option<SpaceId>>,
    extension: Option<PostExtension>,
    content: Option<Content>,
) -> DispatchResult {
    Posts::create_post(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        space_id_opt.unwrap_or(Some(SPACE1)),
        extension.unwrap_or_else(extension_regular_post),
        content.unwrap_or_else(post_content_ipfs),
    )
}

pub(crate) fn _update_post(
    origin: Option<Origin>,
    post_id: Option<PostId>,
    update: Option<PostUpdate>,
) -> DispatchResult {
    Posts::update_post(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        post_id.unwrap_or(POST1),
        update.unwrap_or_else(|| post_update(None, None, None)),
    )
}

pub(crate) fn _move_post_1_to_space_2() -> DispatchResult {
    _move_post(None, None, None)
}

/// Move the post out of this space to nowhere (space = None).
pub(crate) fn _move_post_to_nowhere(post_id: PostId) -> DispatchResult {
    _move_post(None, Some(post_id), Some(None))
}

pub(crate) fn _move_post(
    origin: Option<Origin>,
    post_id: Option<PostId>,
    new_space_id: Option<Option<SpaceId>>,
) -> DispatchResult {
    Posts::move_post(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        post_id.unwrap_or(POST1),
        new_space_id.unwrap_or(Some(SPACE2)),
    )
}

pub(crate) fn _create_default_comment() -> DispatchResult {
    _create_comment(None, None, None, None)
}

pub(crate) fn _create_comment(
    origin: Option<Origin>,
    post_id: Option<PostId>,
    parent_id: Option<Option<PostId>>,
    content: Option<Content>,
) -> DispatchResult {
    _create_post(
        origin,
        Some(None),
        Some(extension_comment(
            parent_id.unwrap_or_default(),
            post_id.unwrap_or(POST1),
        )),
        Some(content.unwrap_or_else(comment_content_ipfs)),
    )
}

pub(crate) fn _update_comment(
    origin: Option<Origin>,
    post_id: Option<PostId>,
    update: Option<PostUpdate>,
) -> DispatchResult {
    _update_post(
        origin,
        Some(post_id.unwrap_or(POST2)),
        Some(update.unwrap_or_else(||
            post_update(None, Some(reply_content_ipfs()), None))
        ),
    )
}

pub(crate) fn _create_default_post_reaction() -> DispatchResult {
    _create_post_reaction(None, None, None)
}

pub(crate) fn _create_default_comment_reaction() -> DispatchResult {
    _create_comment_reaction(None, None, None)
}

pub(crate) fn _create_post_reaction(
    origin: Option<Origin>,
    post_id: Option<PostId>,
    kind: Option<ReactionKind>,
) -> DispatchResult {
    Reactions::create_post_reaction(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        post_id.unwrap_or(POST1),
        kind.unwrap_or_else(reaction_upvote),
    )
}

pub(crate) fn _create_comment_reaction(
    origin: Option<Origin>,
    post_id: Option<PostId>,
    kind: Option<ReactionKind>,
) -> DispatchResult {
    _create_post_reaction(origin, Some(post_id.unwrap_or(2)), kind)
}

pub(crate) fn _update_post_reaction(
    origin: Option<Origin>,
    post_id: Option<PostId>,
    reaction_id: ReactionId,
    kind: Option<ReactionKind>,
) -> DispatchResult {
    Reactions::update_post_reaction(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        post_id.unwrap_or(POST1),
        reaction_id,
        kind.unwrap_or_else(reaction_upvote),
    )
}

pub(crate) fn _update_comment_reaction(
    origin: Option<Origin>,
    post_id: Option<PostId>,
    reaction_id: ReactionId,
    kind: Option<ReactionKind>,
) -> DispatchResult {
    _update_post_reaction(origin, Some(post_id.unwrap_or(2)), reaction_id, kind)
}

pub(crate) fn _delete_post_reaction(
    origin: Option<Origin>,
    post_id: Option<PostId>,
    reaction_id: ReactionId,
) -> DispatchResult {
    Reactions::delete_post_reaction(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        post_id.unwrap_or(POST1),
        reaction_id,
    )
}

pub(crate) fn _delete_comment_reaction(
    origin: Option<Origin>,
    post_id: Option<PostId>,
    reaction_id: ReactionId,
) -> DispatchResult {
    _delete_post_reaction(origin, Some(post_id.unwrap_or(2)), reaction_id)
}

pub(crate) fn _create_default_profile() -> DispatchResult {
    _create_profile(None, None)
}

pub(crate) fn _create_profile(
    origin: Option<Origin>,
    content: Option<Content>
) -> DispatchResult {
    Profiles::create_profile(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        content.unwrap_or_else(profile_content_ipfs),
    )
}

pub(crate) fn _update_profile(
    origin: Option<Origin>,
    content: Option<Content>
) -> DispatchResult {
    Profiles::update_profile(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        ProfileUpdate {
            content,
        },
    )
}

pub(crate) fn _default_follow_account() -> DispatchResult {
    _follow_account(None, None)
}

pub(crate) fn _follow_account(origin: Option<Origin>, account: Option<AccountId>) -> DispatchResult {
    ProfileFollows::follow_account(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT2)),
        account.unwrap_or(ACCOUNT1),
    )
}

pub(crate) fn _default_unfollow_account() -> DispatchResult {
    _unfollow_account(None, None)
}

pub(crate) fn _unfollow_account(origin: Option<Origin>, account: Option<AccountId>) -> DispatchResult {
    ProfileFollows::unfollow_account(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT2)),
        account.unwrap_or(ACCOUNT1),
    )
}

pub(crate) fn _transfer_default_space_ownership() -> DispatchResult {
    _transfer_space_ownership(None, None, None)
}

pub(crate) fn _transfer_space_ownership(
    origin: Option<Origin>,
    space_id: Option<SpaceId>,
    transfer_to: Option<AccountId>,
) -> DispatchResult {
    SpaceOwnership::transfer_space_ownership(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        space_id.unwrap_or(SPACE1),
        transfer_to.unwrap_or(ACCOUNT2),
    )
}

pub(crate) fn _accept_default_pending_ownership() -> DispatchResult {
    _accept_pending_ownership(None, None)
}

pub(crate) fn _accept_pending_ownership(origin: Option<Origin>, space_id: Option<SpaceId>) -> DispatchResult {
    SpaceOwnership::accept_pending_ownership(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT2)),
        space_id.unwrap_or(SPACE1),
    )
}

pub(crate) fn _reject_default_pending_ownership() -> DispatchResult {
    _reject_pending_ownership(None, None)
}

pub(crate) fn _reject_default_pending_ownership_by_current_owner() -> DispatchResult {
    _reject_pending_ownership(Some(Origin::signed(ACCOUNT1)), None)
}

pub(crate) fn _reject_pending_ownership(origin: Option<Origin>, space_id: Option<SpaceId>) -> DispatchResult {
    SpaceOwnership::reject_pending_ownership(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT2)),
        space_id.unwrap_or(SPACE1),
    )
}

/* ---------------------------------------------------------------------------------------------- */

// TODO: fix copy-paste from pallet_roles
/* Roles pallet mocks */

type RoleId = u64;

pub(crate) const ROLE1: RoleId = 1;
pub(crate) const ROLE2: RoleId = 2;

pub(crate) fn default_role_content_ipfs() -> Content {
    Content::IPFS(b"QmRAQB6YaCyidP37UdDnjFY5vQuiBrcqdyoW1CuDgwxkD4".to_vec())
}

/// Permissions Set that includes next permission: ManageRoles
pub(crate) fn permission_set_default() -> Vec<SpacePermission> {
    vec![SP::ManageRoles]
}


pub fn _create_default_role() -> DispatchResult {
    _create_role(None, None, None, None, None)
}

pub fn _create_role(
    origin: Option<Origin>,
    space_id: Option<SpaceId>,
    time_to_live: Option<Option<BlockNumber>>,
    content: Option<Content>,
    permissions: Option<Vec<SpacePermission>>,
) -> DispatchResult {
    Roles::create_role(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        space_id.unwrap_or(SPACE1),
        time_to_live.unwrap_or_default(), // Should return 'None'
        content.unwrap_or_else(default_role_content_ipfs),
        permissions.unwrap_or_else(permission_set_default),
    )
}

pub fn _grant_default_role() -> DispatchResult {
    _grant_role(None, None, None)
}

pub fn _grant_role(
    origin: Option<Origin>,
    role_id: Option<RoleId>,
    users: Option<Vec<User<AccountId>>>,
) -> DispatchResult {
    Roles::grant_role(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        role_id.unwrap_or(ROLE1),
        users.unwrap_or_else(|| vec![User::Account(ACCOUNT2)]),
    )
}

pub fn _delete_default_role() -> DispatchResult {
    _delete_role(None, None)
}

pub fn _delete_role(
    origin: Option<Origin>,
    role_id: Option<RoleId>,
) -> DispatchResult {
    Roles::delete_role(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        role_id.unwrap_or(ROLE1),
    )
}

/* ---------------------------------------------------------------------------------------------- */
// Moderation pallet mocks
// FIXME: remove when linter error is fixed
#[allow(dead_code)]
const REPORT1: ReportId = 1;

pub(crate) fn _report_default_post() -> DispatchResult {
    _report_entity(None, None, None, None)
}

pub(crate) fn _report_entity(
    origin: Option<Origin>,
    entity: Option<EntityId<AccountId>>,
    scope: Option<SpaceId>,
    reason: Option<Content>,
) -> DispatchResult {
    Moderation::report_entity(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        entity.unwrap_or(EntityId::Post(POST1)),
        scope.unwrap_or(SPACE1),
        reason.unwrap_or_else(valid_content_ipfs),
    )
}

pub(crate) fn _suggest_entity_status(
    origin: Option<Origin>,
    entity: Option<EntityId<AccountId>>,
    scope: Option<SpaceId>,
    status: Option<Option<EntityStatus>>,
    report_id_opt: Option<Option<ReportId>>,
) -> DispatchResult {
    Moderation::suggest_entity_status(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        entity.unwrap_or(EntityId::Post(POST1)),
        scope.unwrap_or(SPACE1),
        status.unwrap_or(Some(EntityStatus::Blocked)),
        report_id_opt.unwrap_or(Some(REPORT1)),
    )
}

pub(crate) fn _update_entity_status(
    origin: Option<Origin>,
    entity: Option<EntityId<AccountId>>,
    scope: Option<SpaceId>,
    status_opt: Option<Option<EntityStatus>>,
) -> DispatchResult {
    Moderation::update_entity_status(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        entity.unwrap_or(EntityId::Post(POST1)),
        scope.unwrap_or(SPACE1),
        status_opt.unwrap_or(Some(EntityStatus::Allowed)),
    )
}

pub(crate) fn _delete_entity_status(
    origin: Option<Origin>,
    entity: Option<EntityId<AccountId>>,
    scope: Option<SpaceId>,
) -> DispatchResult {
    Moderation::delete_entity_status(
        origin.unwrap_or_else(|| Origin::signed(ACCOUNT1)),
        entity.unwrap_or(EntityId::Post(POST1)),
        scope.unwrap_or(SPACE1),
    )
}

/*------------------------------------------------------------------------------------------------*/
// Moderation tests

pub(crate) fn block_account_in_space_1() {
    assert_ok!(
            _update_entity_status(
                None,
                Some(EntityId::Account(ACCOUNT1)),
                Some(SPACE1),
                Some(Some(EntityStatus::Blocked))
            )
        );
}

pub(crate) fn block_content_in_space_1() {
    assert_ok!(
            _update_entity_status(
                None,
                Some(EntityId::Content(valid_content_ipfs())),
                Some(SPACE1),
                Some(Some(EntityStatus::Blocked))
            )
        );
}