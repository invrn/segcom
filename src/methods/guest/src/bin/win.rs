use fleetcore::{BaseInputs, BaseJournal};
use risc0_zkvm::guest::env;
use risc0_zkvm::Digest;
use sha2::{Digest as _, Sha256};

fn main() {

    // read the input
    let input: BaseInputs = env::read();
        // Prove that the fleet is not sunk: at least one position remains
    if input.board.is_empty() {
        panic!("Cannot claim win: fleet is completely sunk!");
    }
    // Hash the board using (random || board)
    let mut board_preimage = input.random.as_bytes().to_vec();
    board_preimage.extend_from_slice(&input.board);
    let mut board_hasher = Sha256::new();
    board_hasher.update(board_preimage);
    let board_digest = Digest::from_bytes(board_hasher.finalize().into());

    let output = BaseJournal {
        gameid: input.gameid,
        fleet: input.fleet,
        board: board_digest,
    };
    // write public output to the journal
    env::commit(&output);
}
