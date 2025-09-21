use crate::error::{AppError, AppResult};
use crate::models::validator::Validator;
use sqlx::{Pool, Postgres};

fn validate_contract_address(address: &str) -> Result<(), AppError> {
    let is_valid = address.starts_with("0x")
        && address.len() == 42
        && address.chars().all(|c| c.is_ascii_hexdigit() || c == 'x');
    if !is_valid {
        Err(AppError::BadRequest(
            "Invalid contract address format".into(),
        ))
    } else {
        Ok(())
    }
}

pub async fn get_validator(pool: &Pool<Postgres>, address: &str) -> AppResult<Validator> {
    validate_contract_address(address)?;
    let validator = sqlx::query_as::<_, Validator>(
        "SELECT * FROM validators WHERE LOWER(contract_address) = LOWER($1)",
    )
    .bind(address)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    validator.ok_or_else(|| AppError::NotFound("Validator not found".into()))
}

pub async fn get_validators(pool: &Pool<Postgres>) -> AppResult<Vec<Validator>> {
    let validators = sqlx::query_as::<_, Validator>("SELECT * FROM validators")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(validators)
}

pub async fn get_validator_status(pool: &Pool<Postgres>, address: &str) -> AppResult<bool> {
    validate_contract_address(address)?;
    let status = sqlx::query_scalar::<_, bool>(
        "SELECT is_active FROM validators WHERE LOWER(contract_address) = LOWER($1)",
    )
    .bind(address)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    status.ok_or_else(|| AppError::NotFound("Validator not found".into()))
}
