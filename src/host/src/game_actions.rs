// src/game_actions.rs
use fleetcore::{BaseInputs, Command, FireInputs};
use methods::{FIRE_ELF, JOIN_ELF, REPORT_ELF, WAVE_ELF, WIN_ELF};
use risc0_zkvm::{default_prover, ExecutorEnv, Receipt};

use crate::{send_receipt, unmarshal_data, unmarshal_fire, unmarshal_report, FormData};

pub async fn join_game(idata: FormData) -> String {
    let (gameid, fleetid, board, random) = match unmarshal_data(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };

    // Call a helper function to generate the receipt
    let receipt = match generate_receipt(
        gameid.clone(),
        fleetid.clone(),
        board.clone(),
        random.clone(),
    ) {
        Ok(receipt) => receipt,
        Err(err) => return format!("Failed to generate receipt: {}", err),
    };

    // Send the receipt
    send_receipt(Command::Join, receipt).await
}

// Helper function to generate the receipt
fn generate_receipt(
    gameid: String,
    fleetid: String,
    board: Vec<u8>,
    random: String,
) -> Result<Receipt, String> {
    // Construct BaseInputs to send to the zkVM guest
    let input = BaseInputs {
        gameid,
        fleet: fleetid,
        board,
        random,
    };

    // Set up the zkVM execution environment and write the input
    let env = ExecutorEnv::builder()
        .write(&input)
        .map_err(|err| format!("Failed to write input to executor env: {}", err))?
        .build()
        .map_err(|err| format!("Failed to build executor env: {}", err))?;

    // Obtain the default prover
    let prover = default_prover();

    // Produce a receipt by proving the specified ELF binary
    let prove_info = prover
        .prove(env, JOIN_ELF)
        .map_err(|err| format!("Failed to prove: {}", err))?;

    Ok(prove_info.receipt)
}

pub async fn fire(idata: FormData) -> String {
    let (gameid, fleetid, board, random, targetfleet, x, y) = match unmarshal_fire(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };

    // Convert coordinates to a single position (0-99) for the FireInputs struct
    let pos: u8 = y * 10 + x;

    // Call a helper function to generate the receipt for firing
    let receipt = match generate_fire_receipt(
        gameid.clone(),
        fleetid.clone(),
        board.clone(),
        random.clone(),
        targetfleet.clone(),
        pos,
    ) {
        Ok(receipt) => receipt,
        Err(err) => return format!("Failed to generate receipt: {}", err),
    };

    // Send the receipt
    send_receipt(Command::Fire, receipt).await
}

// Helper function to generate the receipt for firing
fn generate_fire_receipt(
    gameid: String,
    fleetid: String,
    board: Vec<u8>,
    random: String,
    target: String,
    pos: u8,
) -> Result<Receipt, String> {
    // Construct FireInputs to send to the zkVM guest
    let input = FireInputs {
        gameid,
        fleet: fleetid,
        board,
        random,
        target,
        pos,
    };

    // Set up the zkVM execution environment and write the input
    let env = ExecutorEnv::builder()
        .write(&input)
        .map_err(|err| format!("Failed to write input to executor env: {}", err))?
        .build()
        .map_err(|err| format!("Failed to build executor env: {}", err))?;

    // Obtain the default prover
    let prover = default_prover();

    // Produce a receipt by proving the specified ELF binary
    let prove_info = prover
        .prove(env, FIRE_ELF)
        .map_err(|err| format!("Failed to prove: {}", err))?;

    Ok(prove_info.receipt)
}

pub async fn report(idata: FormData) -> String {
    let (gameid, fleetid, board, random, _report, x, y) = match unmarshal_report(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };

    // Convert coordinates to a single position (0-99)
    let pos: u8 = y * 10 + x;

    // Call a helper function to generate the receipt for reporting
    let receipt = match generate_report_receipt(
        gameid.clone(),
        fleetid.clone(),
        board.clone(),
        random.clone(),
        pos,
    ) {
        Ok(receipt) => receipt,
        Err(err) => return format!("Failed to generate receipt: {}", err),
    };

    // Send the receipt
    send_receipt(Command::Report, receipt).await
}

// Helper function to generate the receipt for reporting
fn generate_report_receipt(
    gameid: String,
    fleetid: String,
    board: Vec<u8>,
    random: String,
    pos: u8,
) -> Result<Receipt, String> {
    // Reuse FireInputs
    let input = FireInputs {
        gameid,
        fleet: fleetid,
        board,
        random,
        target: String::new(), // Not used for report
        pos,
    };

    // Set up the zkVM execution environment and write the input
    let env = ExecutorEnv::builder()
        .write(&input)
        .map_err(|err| format!("Failed to write input to executor env: {}", err))?
        .build()
        .map_err(|err| format!("Failed to build executor env: {}", err))?;

    // Obtain the default prover
    let prover = default_prover();

    // Produce a receipt by proving the specified ELF binary
    let prove_info = prover
        .prove(env, REPORT_ELF)
        .map_err(|err| format!("Failed to prove: {}", err))?;

    Ok(prove_info.receipt)
}

pub async fn wave(idata: FormData) -> String {
    let (gameid, fleetid, board, random) = match unmarshal_data(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };
    // Call a helper function to generate the receipt for wave
    let receipt = match generate_wave_receipt(gameid.clone(), fleetid.clone(), board, random) {
        Ok(receipt) => receipt,
        Err(err) => return format!("Failed to generate receipt: {}", err),
    };

    // Send the receipt    // Send the receipt
    send_receipt(Command::Wave, receipt).await
}

fn generate_wave_receipt(
    gameid: String,
    fleetid: String,
    board: Vec<u8>,
    random: String,
) -> Result<Receipt, String> {
    // Construct BaseInputs to send to the zkVM guest
    let input = BaseInputs {
        gameid,
        fleet: fleetid,
        board,
        random,
    };

    // Set up the zkVM execution environment and write the input
    let env = ExecutorEnv::builder()
        .write(&input)
        .map_err(|err| format!("Failed to write input to executor env: {}", err))?
        .build()
        .map_err(|err| format!("Failed to build executor env: {}", err))?;

    // Obtain the default prover
    let prover = default_prover();

    // Produce a receipt by proving the specified ELF binary
    let prove_info = prover
        .prove(env, WAVE_ELF)
        .map_err(|err| format!("Failed to prove: {}", err))?;

    Ok(prove_info.receipt)
}
pub async fn win(idata: FormData) -> String {
    let (gameid, fleetid, board, random) = match unmarshal_data(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };

    // Call a helper function to generate the receipt for win
    let receipt = match generate_win_receipt(
        gameid.clone(),
        fleetid.clone(),
        board.clone(),
        random.clone(),
    ) {
        Ok(receipt) => receipt,
        Err(err) => return format!("Failed to generate receipt: {}", err),
    };

    // Send the receipt
    send_receipt(Command::Win, receipt).await
}

// Helper function to generate the receipt for win
fn generate_win_receipt(
    gameid: String,
    fleetid: String,
    board: Vec<u8>,
    random: String,
) -> Result<Receipt, String> {
    // Construct BaseInputs to send to the zkVM guest
    let input = BaseInputs {
        gameid,
        fleet: fleetid,
        board,
        random,
    };

    // Set up the zkVM execution environment and write the input
    let env = ExecutorEnv::builder()
        .write(&input)
        .map_err(|err| format!("Failed to write input to executor env: {}", err))?
        .build()
        .map_err(|err| format!("Failed to build executor env: {}", err))?;

    // Obtain the default prover
    let prover = default_prover();

    // Produce a receipt by proving the specified ELF binary
    let prove_info = prover
        .prove(env, WIN_ELF)
        .map_err(|err| format!("Failed to prove: {}", err))?;

    Ok(prove_info.receipt)
}
