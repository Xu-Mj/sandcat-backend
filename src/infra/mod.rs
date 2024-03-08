pub(crate) mod db;
pub(crate) mod errors;
pub(crate) mod repositories;

pub trait Validator {
    fn validate(&self) -> Result<(), errors::InfraError>;
}
