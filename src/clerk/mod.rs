//! Clerk: an autonomous `Explorer` that ONLY collects (mines) basic
//! resources -- he never combines anything, and never learns/asks about
//! combination recipes for his own planning. He keeps a running tally of
//! every basic resource type he's mined, so at any moment it's possible
//! to see which resource type is currently his "leader" (most collected).
//!
//! Design notes, since this deliberately drops most of Eco's economy:
//!   - No wallet / coins / regime pricing at all -- collecting is free.
//!   - No recipes / combine planning -- Clerk never builds a
//!     `ComplexResourceRequest` and never sends `CombineResourceRequest`
//!     to a planet.
//!   - Movement is exploration-only: Clerk wanders to unvisited planets
//!     to broaden what he can collect, exactly like Eco's `nearest_unvisited`
//!     strategy, just without a task/recipe driving *where* to go.
//!
//! Module map:
//!   world     - learned planet graph (neighbors + which basics a planet has)
//!   ledger    - running per-basic-resource-type collection counts + leader
//!   planner   - turns (current planet, world model, this-visit mining
//!               progress) into one action per cycle
//!   logging   - every log line Clerk emits, in one place
//!   explorer  - the `Explorer` struct itself + protocol wiring
//!
//! VERIFY against the real `common_game` crate before compiling
//! (see https://unitn-ap-2025.github.io/common-docs/common_game/index.html):
//!   - `BasicResourceType` is `Copy + Eq + Hash + Debug`.
//!   - `BasicResource::get_type()` exists (confirmed used by Eco's `bag.rs`).
//!   - `OrchestratorToExplorer` / `ExplorerToOrchestrator` / `ExplorerToPlanet`
//!     / `PlanetToExplorer` variant shapes match those already used by Eco.

mod explorer;
mod ledger;
mod logging;
mod planner;
mod world;

pub use explorer::{create_clerk, Explorer};