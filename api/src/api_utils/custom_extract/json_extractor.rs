use abi::errors::Error;
use axum::{
    async_trait,
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
};

pub struct JsonExtractor<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for JsonExtractor<T>
where
    axum::Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Error);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();

        // We can use other extractors to provide better rejection messages.
        // For example, here we are using `axum::extract::MatchedPath` to
        // provide a better errors message.
        //
        // Have to run that first since `Json` extraction consumes the request.
        // let path = parts
        //     .extract::<MatchedPath>()
        //     .await
        //     .map(|path| path.as_str().to_owned())
        //     .ok();

        let req = Request::from_parts(parts, body);

        match axum::Json::<T>::from_request(req, state).await {
            Ok(value) => Ok(Self(value.0)),
            // convert the errors from `axum::Json` into whatever we want
            Err(rejection) => Err((
                rejection.status(),
                Error::body_parsing(rejection.body_text()),
            )),
        }
    }
}
