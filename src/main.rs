mod clerk;

use std::thread;
use std::time::Duration;

use crossbeam_channel::unbounded;

use common_game::protocols::orchestrator_explorer::OrchestratorToExplorer;
use common_game::utils::ID;

fn main() {
    env_logger::init();

    // Channels connecting "the orchestrator" (this main function,
    // standing in for the real one) to Clerk.
    let (tx_to_clerk, rx_from_orchestrator) = unbounded::<OrchestratorToExplorer>();
    let (tx_to_orchestrator, rx_from_clerk) = unbounded();

    let clerk_id: ID = 2;
    let starting_planet_id: ID = 100;

    let clerk = clerk::create_clerk(clerk_id, starting_planet_id, rx_from_orchestrator, tx_to_orchestrator);

    // Clerk runs its own loop on its own thread, exactly like the real
    // orchestrator would spawn it.
    let handle = thread::spawn(move || clerk.run());

    // Drain whatever Clerk sends back, just so we can see it happening.
    let listener = thread::spawn(move || {
        while let Ok(msg) = rx_from_clerk.recv() {
            println!("Clerk -> orchestrator: {msg:?}");
        }
    });

    // Kick the AI on.
    tx_to_clerk.send(OrchestratorToExplorer::StartExplorerAI).unwrap();

    // Let it run for a bit so you can watch the collecting tick in the
    // logs (run with RUST_LOG=info to see them).
    thread::sleep(Duration::from_secs(5));

    tx_to_clerk.send(OrchestratorToExplorer::KillExplorer).unwrap();

    handle.join().unwrap();
    drop(tx_to_clerk);
    let _ = listener.join();
}