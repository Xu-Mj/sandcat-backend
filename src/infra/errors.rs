use deadpool_diesel::InteractError;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum InfraError {
    InternalServerError(String),
    NotFound,
}

pub fn adapt_infra_error<E: Error>(err: E) -> InfraError {
    err.as_infra_error()
}

// 自定义错误特征，用来转为基础设施错误
pub trait Error {
    fn as_infra_error(&self) -> InfraError;
}

// 实现Display特征，设置InfraError打印格式
impl Display for InfraError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InfraError::InternalServerError(err_msg) => {
                write!(f, "Internal Server Error{}", err_msg)
            }
            InfraError::NotFound => {
                write!(f, "Not Found")
            }
        }
    }
}

// 将diesel错误转为基础设施错误
impl Error for diesel::result::Error {
    fn as_infra_error(&self) -> InfraError {
        match self {
            diesel::result::Error::NotFound => InfraError::NotFound,
            _ => InfraError::InternalServerError(self.to_string()),
        }
    }
}

// 转换连接池错误
impl Error for deadpool_diesel::PoolError {
    fn as_infra_error(&self) -> InfraError {
        InfraError::InternalServerError(self.to_string())
    }
}

impl Error for InteractError {
    fn as_infra_error(&self) -> InfraError {
        InfraError::InternalServerError(self.to_string())
    }
}
