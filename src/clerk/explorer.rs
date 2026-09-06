//! The `Explorer` struct itself: protocol handling (same shape as
//! Eco's) plus the wiring that drives Clerk's collection loop every
//! `ai_step`. Unlike Eco, there is no economy (no wallet/regime/costs)
//! and no combine capability at all.

use std::collections::HashSet;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, TryRecvError};

use common_game::components::resource::{BasicResourceType, ComplexResourceType, GenericResource};
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;

use super::ledger::Ledger;
use super::logging;
use super::planner::{Action, Planner};
use super::world::WorldModel;

#[must_use]
pub fn create_clerk(
    explorer_id: ID,
    starting_planet_id: ID,
    rx_orchestrator: Receiver<OrchestratorToExplorer>,
    tx_orchestrator: Sender<ExplorerToOrchestrator<GenericResource>>,
) -> Explorer {
    Explorer::new(explorer_id, starting_planet_id, rx_orchestrator, tx_orchestrator)
}

/// Clerk's link to whichever planet he's currently on. `None` until the
/// first successful `MoveToPlanet` -- same reasoning as Eco's `PlanetLink`.
#[derive(Default)]
struct PlanetLink {
    to_planet: Option<Sender<ExplorerToPlanet>>,
    from_planet: Option<Receiver<PlanetToExplorer>>,
}

pub struct Explorer {
    id: ID,

    from_orchestrator: Receiver<OrchestratorToExplorer>,
    to_orchestrator: Sender<ExplorerToOrchestrator<GenericResource>>,

    planet_link: PlanetLink,
    current_planet_id: ID,

    ai_active: bool,
    should_stop: bool,

    // ---- Clerk's own collection state (no protocol involvement) ----
    world: WorldModel,
    ledger: Ledger,
    /// Which basic resource types Clerk has already mined *on this visit*
    /// to the current planet -- reset whenever he arrives somewhere new,
    /// or when the whole known map has been explored and he loops back
    /// to keep collecting where he stands. Drives "sample everything
    /// available here, then move on" instead of mining the same type
    /// forever.
    mined_here_this_visit: HashSet<BasicResourceType>,
}

impl Explorer {
    fn new(
        id: ID,
        starting_planet_id: ID,
        from_orchestrator: Receiver<OrchestratorToExplorer>,
        to_orchestrator: Sender<ExplorerToOrchestrator<GenericResource>>,
    ) -> Self {
        let mut world = WorldModel::default();
        world.visited.insert(starting_planet_id);

        Self {
            id,
            from_orchestrator,
            to_orchestrator,
            planet_link: PlanetLink::default(),
            current_planet_id: starting_planet_id,
            ai_active: false,
            should_stop: false,
            world,
            ledger: Ledger::default(),
            mined_here_this_visit: HashSet::new(),
        }
    }

    /// Current leader (resource type, count) if Clerk has mined
    /// anything yet. Exposed mainly for tests / a hosting `main.rs` --
    /// the log line (`logging::new_leading_resource`) is the primary way
    /// this information surfaces during a run.
    pub fn current_leader(&self) -> Option<(BasicResourceType, u32)> {
        self.ledger.leader()
    }

    /// Main loop. Call this from the thread the orchestrator's `main.rs` spawns.
    pub fn run(mut self) {
        let _ = self.to_orchestrator.send(ExplorerToOrchestrator::CurrentPlanetResult {
            explorer_id: self.id,
            planet_id: self.current_planet_id,
        });

        while !self.should_stop {
            loop {
                match self.from_orchestrator.try_recv() {
                    Ok(msg) => self.handle_orchestrator_message(msg),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        logging::orchestrator_channel_closed(self.id);
                        self.should_stop = true;
                        break;
                    }
                }
            }

            if self.should_stop {
                break;
            }

            if self.ai_active {
                self.ai_step();
            }

            std::thread::sleep(Duration::from_millis(100));
        }

        logging::shutting_down(self.id);
    }

    // ==================== Orchestrator -> Explorer ====================

    fn handle_orchestrator_message(&mut self, msg: OrchestratorToExplorer) {
        match msg {
            OrchestratorToExplorer::KillExplorer => {
                logging::kill_received(self.id);
                self.should_stop = true;
                let _ = self.to_orchestrator.send(
                    ExplorerToOrchestrator::KillExplorerResult { explorer_id: self.id },
                );
            }

            OrchestratorToExplorer::StartExplorerAI => {
                self.ai_active = true;
                let _ = self.to_orchestrator.send(
                    ExplorerToOrchestrator::StartExplorerAIResult { explorer_id: self.id },
                );
            }

            OrchestratorToExplorer::StopExplorerAI => {
                self.ai_active = false;
                let _ = self.to_orchestrator.send(
                    ExplorerToOrchestrator::StopExplorerAIResult { explorer_id: self.id },
                );
            }

            OrchestratorToExplorer::ResetExplorerAI => {
                self.ai_active = false;
                self.world.visited = HashSet::from([self.current_planet_id]);
                self.mined_here_this_visit.clear();
                // Deliberately NOT resetting the ledger here -- the
                // collection tally is Clerk's persistent life's work,
                // not AI-loop state. Swap this comment out if a reset
                // should mean "wipe the collection history" too.
                let _ = self.to_orchestrator.send(
                    ExplorerToOrchestrator::ResetExplorerAIResult { explorer_id: self.id },
                );
            }

            OrchestratorToExplorer::MoveToPlanet { sender_to_new_planet, planet_id } => {
                match sender_to_new_planet {
                    Some(new_to_planet) => {
                        logging::moved_to_planet(self.id, planet_id);
                        self.planet_link = PlanetLink {
                            to_planet: Some(new_to_planet),
                            from_planet: None, // orchestrator never delivers this, same as Eco
                        };
                        self.current_planet_id = planet_id;
                        self.world.visited.insert(planet_id);
                        self.mined_here_this_visit.clear();
                    }
                    None => {
                        logging::move_rejected(self.id, planet_id);
                    }
                }

                let _ = self.to_orchestrator.send(ExplorerToOrchestrator::MovedToPlanetResult {
                    explorer_id: self.id,
                    planet_id: self.current_planet_id,
                });
            }

            OrchestratorToExplorer::CurrentPlanetRequest => {
                let _ = self.to_orchestrator.send(ExplorerToOrchestrator::CurrentPlanetResult {
                    explorer_id: self.id,
                    planet_id: self.current_planet_id,
                });
            }

            OrchestratorToExplorer::SupportedResourceRequest => {
                let supported_resources = self.query_supported_resources();
                let _ = self.to_orchestrator.send(ExplorerToOrchestrator::SupportedResourceResult {
                    explorer_id: self.id,
                    supported_resources,
                });
            }

            OrchestratorToExplorer::SupportedCombinationRequest => {
                // Clerk truthfully reports what the *planet* supports,
                // even though he'll never issue a CombineResourceRequest
                // himself -- this is informational, not a capability
                // claim about Clerk.
                let combination_list = self.query_supported_combinations();
                let _ = self.to_orchestrator.send(ExplorerToOrchestrator::SupportedCombinationResult {
                    explorer_id: self.id,
                    combination_list,
                });
            }

            OrchestratorToExplorer::GenerateResourceRequest { to_generate } => {
                let generated = self.request_generate_resource(to_generate);
                let _ = self.to_orchestrator.send(ExplorerToOrchestrator::GenerateResourceResponse {
                    explorer_id: self.id,
                    generated,
                });
            }

            OrchestratorToExplorer::CombineResourceRequest { .. } => {
                // Clerk cannot combine, full stop -- refuse immediately
                // without ever contacting the planet.
                logging::combine_refused(self.id);
                let _ = self.to_orchestrator.send(ExplorerToOrchestrator::CombineResourceResponse {
                    explorer_id: self.id,
                    generated: Err("Clerk cannot combine resources".to_string()),
                });
            }

            OrchestratorToExplorer::NeighborsResponse { neighbors } => {
                self.world.record_neighbors(self.current_planet_id, neighbors);
            }

            _ => {
                logging::unknown_message(self.id);
            }
        }
    }

    // ==================== Explorer -> Planet ====================

    fn query_supported_resources(&mut self) -> HashSet<BasicResourceType> {
        let Some(tx) = &self.planet_link.to_planet else {
            logging::no_planet_link(self.id);
            return HashSet::new();
        };
        if tx.send(ExplorerToPlanet::SupportedResourceRequest { explorer_id: self.id }).is_err() {
            logging::planet_channel_gone(self.id);
            return HashSet::new();
        }
        let result = match self.await_planet_response() {
            Some(PlanetToExplorer::SupportedResourceResponse { resource_list }) => resource_list,
            Some(PlanetToExplorer::Stopped) => {
                logging::planet_stopped(self.id);
                HashSet::new()
            }
            _ => HashSet::new(),
        };
        self.world.record_resources(self.current_planet_id, result.clone());
        result
    }

    fn query_supported_combinations(&mut self) -> HashSet<ComplexResourceType> {
        let Some(tx) = &self.planet_link.to_planet else {
            return HashSet::new();
        };
        if tx.send(ExplorerToPlanet::SupportedCombinationRequest { explorer_id: self.id }).is_err() {
            return HashSet::new();
        }
        match self.await_planet_response() {
            Some(PlanetToExplorer::SupportedCombinationResponse { combination_list }) => combination_list,
            _ => HashSet::new(),
        }
    }

    fn request_generate_resource(&mut self, resource: BasicResourceType) -> Result<(), String> {
        let Some(tx) = &self.planet_link.to_planet else {
            return Err("not currently linked to a planet".to_string());
        };
        if tx
            .send(ExplorerToPlanet::GenerateResourceRequest { explorer_id: self.id, resource })
            .is_err()
        {
            return Err("planet unreachable".to_string());
        }

        match self.await_planet_response() {
            Some(PlanetToExplorer::GenerateResourceResponse { resource: Some(r) }) => {
                let mined_type = r.get_type();
                self.mined_here_this_visit.insert(mined_type);
                if let Some((leader, count)) = self.ledger.record(mined_type) {
                    logging::new_leading_resource(self.id, leader, count);
                }
                let total = *self.ledger.counts().get(&mined_type).unwrap_or(&0);
                logging::mined(self.id, mined_type, total);
                Ok(())
            }
            Some(PlanetToExplorer::GenerateResourceResponse { resource: None }) => {
                Err("planet could not generate that resource".to_string())
            }
            Some(PlanetToExplorer::Stopped) => Err("planet is stopped".to_string()),
            _ => Err("no response from planet".to_string()),
        }
    }

    /// "Energy Cell Availability" -- explorer-initiated only, no
    /// orchestrator involvement in the real protocol. Kept for parity
    /// with Eco even though Clerk's own planning doesn't use it yet.
    pub fn query_available_energy_cells(&self) -> Option<ID> {
        let tx = self.planet_link.to_planet.as_ref()?;
        if tx.send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: self.id }).is_err() {
            return None;
        }
        match self.await_planet_response() {
            Some(PlanetToExplorer::AvailableEnergyCellResponse { available_cells }) => Some(available_cells),
            _ => None,
        }
    }

    /// Blocks briefly for a single reply from the current planet.
    /// Returns `None` on timeout.
    fn await_planet_response(&self) -> Option<PlanetToExplorer> {
        let rx = self.planet_link.from_planet.as_ref()?;
        rx.recv_timeout(Duration::from_secs(2)).ok()
    }

    // ==================== Autonomous AI: pure collecting ====================

    fn ai_step(&mut self) {
        let action = Planner::next_action(self.current_planet_id, &self.world, &self.mined_here_this_visit);
        self.execute(action);
    }

    fn execute(&mut self, action: Action) {
        match action {
            Action::QueryResourcesHere => {
                let _ = self.query_supported_resources();
            }
            Action::Mine(resource) => {
                if let Err(e) = self.request_generate_resource(resource) {
                    logging::mine_failed(self.id, resource, &e);
                }
            }
            Action::ExploreTowards(dst) => {
                let _ = self.to_orchestrator.send(ExplorerToOrchestrator::TravelToPlanetRequest {
                    explorer_id: self.id,
                    current_planet_id: self.current_planet_id,
                    dst_planet_id: dst,
                });
            }
            Action::RequestNeighbors => {
                let _ = self.to_orchestrator.send(ExplorerToOrchestrator::NeighborsRequest {
                    explorer_id: self.id,
                    current_planet_id: self.current_planet_id,
                });
            }
            Action::RestartMiningRound => {
                logging::round_restarted(self.id, self.current_planet_id);
                self.mined_here_this_visit.clear();
            }
        }
    }
}