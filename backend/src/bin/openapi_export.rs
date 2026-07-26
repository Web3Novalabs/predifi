//! `predifi-openapi` — dump the OpenAPI spec to stdout or a file (#1381).
//!
//! Lets tooling (TypeScript client generation, Postman, linters) read the spec
//! without booting the server and its dependencies:
//!
//! ```text
//! cargo run --bin predifi-openapi                       # stdout
//! cargo run --bin predifi-openapi -- --out openapi.json # write to a file
//! ```

use std::process::ExitCode;

use predifi_backend::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() -> ExitCode {
    let mut out_path: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            "-o" | "--out" => match args.next() {
                Some(path) => out_path = Some(path),
                None => {
                    eprintln!("error: --out requires a path");
                    return ExitCode::from(2);
                }
            },
            other => {
                eprintln!("error: unknown argument {other:?} (see --help)");
                return ExitCode::from(2);
            }
        }
    }

    let spec = match serde_json::to_string_pretty(&ApiDoc::openapi()) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("failed to serialize OpenAPI spec: {e}");
            return ExitCode::FAILURE;
        }
    };

    match out_path {
        Some(path) => {
            if let Err(e) = std::fs::write(&path, format!("{spec}\n")) {
                eprintln!("failed to write {path}: {e}");
                return ExitCode::FAILURE;
            }
            eprintln!("wrote OpenAPI spec to {path}");
        }
        None => println!("{spec}"),
    }

    ExitCode::SUCCESS
}

fn print_help() {
    println!("predifi-openapi — print the PrediFi OpenAPI 3.x specification as JSON");
    println!();
    println!("USAGE:");
    println!("    cargo run --bin predifi-openapi -- [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -o, --out PATH    Write the spec to PATH instead of stdout");
    println!("    -h, --help        Print this help message and exit");
}
