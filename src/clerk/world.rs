//! What Clerk has learned about the planet graph so far. Populated
//! lazily from `NeighborsResponse` / resource query results -- never
//! assumed known up front. Trimmed from Eco's `world.rs`: Clerk never
//! chases a *specific* resource across the map (he mines whatever's on
//! whichever planet he's standing on), so there's no "nearest known
//! source of X" bookkeeping here -- only "nearest planet I haven't
//! visited yet", used purely to broaden what he can collect.

use std::collections::{HashMap, HashSet, VecDeque};

use common_game::components::resource::BasicResourceType;
use common_game::utils::ID;

#[derive(Debug, Default)]
pub struct WorldModel {
    pub neighbors: HashMap<ID, Vec<ID>>,
    pub resources: HashMap<ID, HashSet<BasicResourceType>>,
    pub visited: HashSet<ID>,
}

impl WorldModel {
    pub fn record_neighbors(&mut self, planet: ID, neighbors: Vec<ID>) {
        self.neighbors.insert(planet, neighbors);
    }

    pub fn record_resources(&mut self, planet: ID, resources: HashSet<BasicResourceType>) {
        self.resources.insert(planet, resources);
    }

    /// Nearest planet we haven't visited yet -- used to decide where to
    /// wander next once the current planet's resource set has all been
    /// sampled at least once this visit.
    pub fn nearest_unvisited(&self, from: ID) -> Option<Vec<ID>> {
        self.bfs_to(from, |p| p != from && !self.visited.contains(&p))
    }

    fn bfs_to(&self, from: ID, goal: impl Fn(ID) -> bool) -> Option<Vec<ID>> {
        if goal(from) {
            return Some(vec![from]);
        }
        let mut queue = VecDeque::new();
        let mut came_from: HashMap<ID, ID> = HashMap::new();
        let mut seen = HashSet::new();
        queue.push_back(from);
        seen.insert(from);

        while let Some(cur) = queue.pop_front() {
            let Some(neighbors) = self.neighbors.get(&cur) else { continue };
            for &n in neighbors {
                if seen.insert(n) {
                    came_from.insert(n, cur);
                    if goal(n) {
                        let mut path = vec![n];
                        let mut walk = n;
                        while let Some(&p) = came_from.get(&walk) {
                            path.push(p);
                            walk = p;
                        }
                        path.reverse();
                        return Some(path);
                    }
                    queue.push_back(n);
                }
            }
        }
        None
    }
}