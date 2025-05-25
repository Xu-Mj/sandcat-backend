use tonic::Status;

mod friend;
mod group;
mod msg;
mod user;

#[allow(clippy::result_large_err)]
pub trait Validator {
    fn validate(&self) -> Result<(), Status>;
}
