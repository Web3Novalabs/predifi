use std::process::ExitCode;

use predifi_backend::{
    config::Config,
    db::create_pool,
    migration_checks::{CheckOptions, MigrationChecker},
};

#[tokio::main]
async fn main() -> ExitCode {
    dotenvy::dotenv().ok();

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("failed to load configuration: {error}");
            return ExitCode::FAILURE;
        }
    };

    let pool = match create_pool(&config).await {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!("failed to connect to database: {error}");
            return ExitCode::FAILURE;
        }
    };

    let options = CheckOptions::default();
    match MigrationChecker::new(pool).run(&options).await {
        Ok(report) => {
            print!("{}", report.summary());
            if report
                .dry_run
                .as_ref()
                .is_some_and(|result| !result.success)
                || report.has_breaking_changes
            {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("migration safety check failed: {error}");
            ExitCode::FAILURE
        }
    }
}
