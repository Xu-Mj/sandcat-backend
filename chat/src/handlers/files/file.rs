use crate::domain::model::user::UserError;
use crate::utils::PathExtractor;
use axum::extract::Multipart;
use axum::http::HeaderMap;
use nanoid::nanoid;

pub async fn upload(mut multipart: Multipart) -> Result<String, UserError> {
    // let mut path = String::new();
    let mut filename = String::new();
    if let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| UserError::InternalServerError(err.to_string()))?
    {
        filename = field.file_name().unwrap_or_default().to_string();

        filename = format!("{}-{}", nanoid!(), filename);
        // 校验文件类型（MIME类型）

        // 生成新的随机文件名
        let path = format!("D:\\Projects\\rust\\im-backend\\upload/{}", filename);

        // 将文件保存到磁盘
        let data = field
            .bytes()
            .await
            .map_err(|_e| UserError::InternalServerError(_e.to_string()))?;
        tokio::fs::write(path, &data)
            .await
            .map_err(|err| UserError::InternalServerError(err.to_string()))?;
    }

    Ok(filename)
}

pub async fn get_file_by_name(
    // PathWithAuthExtractor(filename): PathWithAuthExtractor<String>,
    PathExtractor(filename): PathExtractor<String>,
) -> Result<(HeaderMap, Vec<u8>), UserError> {
    let bytes = std::fs::read(format!(
        "D:\\Projects\\rust\\im-backend\\upload/{}",
        filename
    ))
    .map_err(|err| UserError::InternalServerError(err.to_string()))?;
    let mut headers = HeaderMap::new();
    headers.insert(
        "Cache-Control",
        "private, max-age=31536000".parse().unwrap(),
    );
    Ok((headers, bytes))
}
