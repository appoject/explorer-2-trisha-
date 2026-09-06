//! Every log line Clerk emits, defined once here so wording stays
//! consistent and the call sites in `explorer.rs` stay short.

use common_game::components::resource::BasicResourceType;
use common_game::utils::ID;

// ---- Protocol / lifecycle ----

pub(crate) fn orchestrator_channel_closed(id: ID) {
    log::warn!("Explorer {id}: orchestrator channel closed");
}

pub(crate) fn shutting_down(id: ID) {
    log::info!("Explorer {id} shutting down");
}

pub(crate) fn kill_received(id: ID) {
    log::info!("Explorer {id} received KillExplorer");
}

pub(crate) fn moved_to_planet(id: ID, planet_id: ID) {
    log::info!("Explorer {id}: moved to planet {planet_id}");
}

pub(crate) fn move_rejected(id: ID, planet_id: ID) {
    log::warn!("Explorer {id}: move to planet {planet_id} was rejected");
}

pub(crate) fn unknown_message(id: ID) {
    log::error!("Explorer {id}: error unknown received message");
}

pub(crate) fn no_planet_link(id: ID) {
    log::debug!("Explorer {id}: no planet link yet");
}

pub(crate) fn planet_channel_gone(id: ID) {
    log::warn!("Explorer {id}: planet channel gone");
}

pub(crate) fn planet_stopped(id: ID) {
    log::debug!("Explorer {id}: planet is stopped");
}

// ---- Collecting ----

pub(crate) fn mined(id: ID, resource: BasicResourceType, total_of_type: u32) {
    log::info!("Explorer {id}: mined {resource:?} (now have {total_of_type} of it)");
}

pub(crate) fn mine_failed(id: ID, resource: BasicResourceType, reason: &str) {
    log::debug!("Explorer {id}: mine {resource:?} failed: {reason}");
}

pub(crate) fn combine_refused(id: ID) {
    log::debug!("Explorer {id}: asked to combine, but Clerk can't combine resources");
}

/// The headline log line -- fires only when the *identity* of the
/// most-collected resource type changes, not on every mine.
pub(crate) fn new_leading_resource(id: ID, resource: BasicResourceType, count: u32) {
    log::info!("Explorer {id}: {resource:?} is now the most-collected resource ({count} collected)");
}

pub(crate) fn round_restarted(id: ID, planet_id: ID) {
    log::debug!(
        "Explorer {id}: known map fully explored, restarting mining round on planet {planet_id}"
    );
}