use fleetcore::{BaseInputs, BaseJournal};
use risc0_zkvm::guest::env;
use risc0_zkvm::Digest;
use sha2::{Digest as _, Sha256};

fn main() {
    // read the input
    let input: BaseInputs = env::read();

    // Validate the fleet positioning
    // Ensure the fleet is valid
    if !is_valid_fleet(&input.board) {
        panic!("Invalid fleet positioning!");
    }

    // Hashing
    // Combine the random string and board bytes to form the preimage
    let mut preimage = input.random.as_bytes().to_vec();
    preimage.extend_from_slice(&input.board);

    // Compute the hash using SHA256 over (random || board)
    let mut hasher = Sha256::new();
    hasher.update(preimage);
    let hash = hasher.finalize();
    // Convert the hash to a 32-byte array
    let hash: [u8; 32] = hash.into();
    // Convert the hash to a Digest
    let hash = Digest::from_bytes(hash);
    println!(
        "DEBUG: Next Board in guest to report for fleet {}: {:?}",
        input.fleet, hash
    );

    let output = BaseJournal {
        gameid: input.gameid,
        fleet: input.fleet,
        board: hash,
    };

    env::commit(&output);
}

fn is_valid_fleet(board: &[u8]) -> bool {
    use std::collections::HashSet;
    // --- Simple debug flag for quick testing ---
    const DEBUG_ALLOW_3_SHIP: bool = true; // Set to true for debug mode

    if DEBUG_ALLOW_3_SHIP {
        if board.len() == 3 {
            let mut seen = HashSet::new();
            for &pos in board {
                if pos > 99 || !seen.insert(pos) {
                    return false;
                }
            }
            // Check if the 3 cells are contiguous horizontally or vertically
            let mut sorted = board.to_vec();
            sorted.sort_unstable();
            // Horizontal
            if sorted[1] == sorted[0] + 1 && sorted[2] == sorted[1] + 1 && sorted[0] / 10 == sorted[1] / 10 && sorted[1] / 10 == sorted[2] / 10 {
                return true;
            }
            // Vertical
            if sorted[1] == sorted[0] + 10 && sorted[2] == sorted[1] + 10 && sorted[0] % 10 == sorted[1] % 10 && sorted[1] % 10 == sorted[2] % 10 {
                return true;
            }
            return false;
        }
    }
    // --- End debug flag ---

    if board.len() != 18 {
        return false;
    }

    let mut seen = HashSet::new();
    for &pos in board {
        if pos > 99 || !seen.insert(pos) {
            return false;
        }
    }

    let mut positions = board.to_vec();
    positions.sort_unstable();

    let mut sizes = Vec::new();
    let mut used = vec![false; positions.len()];

    for idx in 0..positions.len() {
        if used[idx] {
            continue;
        }
        let start = positions[idx];
        let mut ship = vec![start];
        used[idx] = true;

        // Try to grow horizontally
        let mut next = start + 1;
        while let Some(pos_idx) = positions.iter().enumerate().find_map(|(k, &p)| {
            if p == next && !used[k] {
                Some(k)
            } else {
                None
            }
        }) {
            // Ensure same row
            if next / 10 != start / 10 {
                break;
            }
            ship.push(next);
            used[pos_idx] = true;
            next += 1;
        }

        // If only one position, try vertical
        if ship.len() == 1 {
            let mut next = start + 10;
            while let Some(pos_idx) = positions.iter().enumerate().find_map(|(k, &p)| {
                if p == next && !used[k] {
                    Some(k)
                } else {
                    None
                }
            }) {
                // Ensure same column
                if next % 10 != start % 10 {
                    break;
                }
                ship.push(next);
                used[pos_idx] = true;
                next += 10;
            }
        }

        // If still only one position, it's a submarine
        sizes.push(ship.len());
    }

    sizes.sort_unstable();
    sizes == vec![1, 1, 2, 2, 3, 4, 5]
}
