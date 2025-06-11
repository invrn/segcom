use fleetcore::{FireInputs, FireJournal};
use risc0_zkvm::guest::env;
use risc0_zkvm::Digest;
use sha2::{Digest as _, Sha256};

fn main() {
    // Read the input
    let input: FireInputs = env::read();

    // Validate the position
    if input.pos >= 100 {
        panic!(
            "Invalid shot position: {}. Must be within the 10x10 board (0-99).",
            input.pos
        );
    }

    // Check if fleet is not sunk
    // Prove that the fleet is not sunk: at least one position remains
    if input.board.is_empty() {
        panic!("Cannot fire: fleet is completely sunk!");
    }

    // Check if target is different from own fleet
    if input.fleet == input.target {
        panic!("Cannot fire at own fleet!");
    }
    // Hash the board before the shot using (random || board)
    let mut board_preimage = input.random.as_bytes().to_vec();
    board_preimage.extend_from_slice(&input.board);
    let mut board_hasher = Sha256::new();
    board_hasher.update(board_preimage);
    let board_digest = Digest::from_bytes(board_hasher.finalize().into());

    // Create the output journal
    let output = FireJournal {
        gameid: input.gameid,
        fleet: input.fleet,
        board: board_digest,
        target: input.target,
        pos: input.pos,
    };

    // Write public output to the journal
    env::commit(&output);
}
