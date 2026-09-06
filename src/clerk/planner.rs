//! Turns (current planet, world model, this-visit mining progress) into
//! one action per cycle. Recomputed every cycle (receding horizon), same
//! spirit as Eco's planner, but with no task/recipe to chase -- Clerk
//! just wants to sample every basic resource type available wherever he
//! is, then wander to broaden the map.

use std::collections::HashSet;

use common_game::components::resource::BasicResourceType;
use common_game::utils::ID;

use super::world::WorldModel;

#[derive(Debug, Clone)]
pub enum Action {
    /// Current planet's resource set isn't known yet -- go ask it directly.
    QueryResourcesHere,
    /// Mine this basic resource type from the current planet.
    Mine(BasicResourceType),
    /// Head towards this (adjacent, per BFS) planet to reach an unvisited one.
    ExploreTowards(ID),
    /// Current planet's neighbor list isn't known yet -- go ask for it.
    RequestNeighbors,
    /// The whole known map has been explored already -- clear this-visit
    /// progress and go around again on the current planet.
    RestartMiningRound,
}

pub struct Planner;

impl Planner {
    pub fn next_action(
        current_planet: ID,
        world: &WorldModel,
        mined_here_this_visit: &HashSet<BasicResourceType>,
    ) -> Action {
        match world.resources.get(&current_planet) {
            None => Action::QueryResourcesHere,
            Some(available) if available.is_empty() => {
                Self::explore_or_request(current_planet, world)
            }
            Some(available) => match available.iter().find(|r| !mined_here_this_visit.contains(*r)) {
                Some(&r) => Action::Mine(r),
                // Sampled everything available here this visit --
                // broaden the map if there's anywhere left to broaden to.
                None => Self::explore_or_request(current_planet, world),
            },
        }
    }

    fn explore_or_request(current_planet: ID, world: &WorldModel) -> Action {
        match world.neighbors.get(&current_planet) {
            None => Action::RequestNeighbors,
            Some(_) => match world.nearest_unvisited(current_planet) {
                Some(path) if path.len() > 1 => Action::ExploreTowards(path[1]),
                // No unvisited planet reachable from here with what we
                // currently know -- rather than idling, keep collecting
                // on the planet we're already on.
                _ => Action::RestartMiningRound,
            },
        }
    }
}