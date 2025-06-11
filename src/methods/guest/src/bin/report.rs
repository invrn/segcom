use fleetcore::{FireInputs, ReportJournal};
use risc0_zkvm::guest::env;
use risc0_zkvm::Digest;
use sha2::{Digest as ShaDigestTrait, Sha256};

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

    // Check if the shot is a hit
    let is_hit = input.board.contains(&input.pos);
    // Print the board being passed for debugging
    println!(
        "DEBUG: Board in guest to report for fleet {}: {:?}",
        input.fleet, input.board
    );

    // Remove the hit position from the board for next_board (if hit)
    let mut next_board = input.board.clone();
    if is_hit {
        next_board.retain(|&p| p != input.pos);
    }

    println!(
        "DEBUG: Next Board in guest to report for fleet {}: {:?}",
        input.fleet, next_board
    );

    let report_str = if is_hit { "Hit" } else { "Miss" };
    // Hash the board before the shot using (random || board)
    let mut board_preimage = input.random.as_bytes().to_vec();
    board_preimage.extend_from_slice(&input.board);
    let mut board_hasher = Sha256::new();
    board_hasher.update(board_preimage);
    let board_digest = Digest::from_bytes(board_hasher.finalize().into());

    // Hash the board after the shot using (random || next_board)
    let mut next_board_preimage = input.random.as_bytes().to_vec();
    next_board_preimage.extend_from_slice(&next_board);
    let mut next_board_hasher = Sha256::new();
    next_board_hasher.update(next_board_preimage);
    let next_board_digest = Digest::from_bytes(next_board_hasher.finalize().into());

    println!(
        "DEBUG: report hashes {}: {:?} {:?}",
        input.fleet, board_digest, next_board_digest
    );
    
    let output = ReportJournal {
        gameid: input.gameid,
        fleet: input.fleet,
        report: report_str.to_string(),
        pos: input.pos,
        board: board_digest,
        next_board: next_board_digest,
    };

    // Write public output to the journal
    env::commit(&output);
}
