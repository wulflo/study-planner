//! Verifica una firma de release contra la clave pública embebida.
//!
//! Uso:
//!   cargo run -q -p study-planner-updater --example verify_release -- \
//!     <artefacto> <firma> <versión> [clave-pública]
//!
//! Sin `clave-pública` usa `UPDATE_PUBLIC_KEY`, la misma que embebe la app.
//! Sale con código 1 si la firma, la versión o el archivo no coinciden.

use std::env;
use std::fs;
use std::process::ExitCode;

use study_planner_updater::{UPDATE_PUBLIC_KEY, sha256_hex, verify_artifact};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 3 || args.len() > 4 {
        eprintln!("uso: verify_release <artefacto> <firma> <versión> [clave-pública]");
        return ExitCode::FAILURE;
    }

    let artifact_path = &args[0];
    let signature_path = &args[1];
    let version = &args[2];
    let public_key = if let Some(path) = args.get(3) {
        match fs::read_to_string(path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("no se pudo leer la clave pública: {error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        UPDATE_PUBLIC_KEY.to_string()
    };

    let artifact = match fs::read(artifact_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("no se pudo leer el artefacto: {error}");
            return ExitCode::FAILURE;
        }
    };
    let signature = match fs::read_to_string(signature_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("no se pudo leer la firma: {error}");
            return ExitCode::FAILURE;
        }
    };

    let sha256 = sha256_hex(&artifact);
    match verify_artifact(&public_key, &artifact, &signature, version, &sha256) {
        Ok(()) => {
            println!("firma válida para la versión {version}");
            println!("sha256: {sha256}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("firma inválida: {error}");
            ExitCode::FAILURE
        }
    }
}
