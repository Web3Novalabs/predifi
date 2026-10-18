//! `predifi-openapi` — dump the OpenAPI spec to stdout or a file (#1381).
//!
//! Lets tooling (TypeScript client generation, Postman, linters) read the spec
//! without booting the server and its dependencies:
//!
//! ```text
//! cargo run --bin predifi-openapi                       # stdout
//! cargo run --bin predifi-openapi -- --out openapi.json # write to a file
//! cargo run --bin predifi-openapi -- --check openapi.json # verify spec is up to date
//! ```

use std::process::ExitCode;

use predifi_backend::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() -> ExitCode {
    let mut out_path: Option<String> = None;
    let mut check_path: Option<String> = None;

    let mut args = std::env::args().skip(1).peekable();
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
            "-c" | "--check" => {
                if let Some(next_arg) = args.peek() {
                    if !next_arg.starts_with('-') {
                        check_path = Some(args.next().unwrap());
                    } else {
                        check_path = Some("openapi.json".to_string());
                    }
                } else {
                    check_path = Some("openapi.json".to_string());
                }
            }
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

    if let Some(path) = check_path {
        let expected = format!("{spec}\n");
        let existing = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("error: could not read OpenAPI spec file at '{path}': {e}");
                eprintln!("Run `cargo run --bin predifi-openapi -- --out {path}` to generate it.");
                return ExitCode::FAILURE;
            }
        };

        let normalized_existing = existing.replace("\r\n", "\n");
        let normalized_expected = expected.replace("\r\n", "\n");

        if normalized_existing != normalized_expected {
            eprintln!("error: OpenAPI spec file at '{path}' is out of date.");
            eprintln!("Run `cargo run --bin predifi-openapi -- --out {path}` to regenerate it.");
            return ExitCode::FAILURE;
        }

        eprintln!("OpenAPI spec at '{path}' is up to date.");
        return ExitCode::SUCCESS;
    }

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
    println!("predifi-openapi — print or check the PrediFi OpenAPI 3.x specification as JSON");
    println!();
    println!("USAGE:");
    println!("    cargo run --bin predifi-openapi -- [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -o, --out PATH          Write the spec to PATH instead of stdout");
    println!("    -c, --check [PATH]      Verify that PATH (default: openapi.json) matches the current spec");
    println!("    -h, --help              Print this help message and exit");
}
