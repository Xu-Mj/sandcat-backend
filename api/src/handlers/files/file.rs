use axum::extract::Multipart;
use axum::http::HeaderMap;
use nanoid::nanoid;

use abi::errors::Error;
use utils::custom_extract::PathExtractor;

pub async fn upload(mut multipart: Multipart) -> Result<String, Error> {
    // let mut path = String::new();
    let mut filename = String::new();
    if let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| Error::InternalServer(err.to_string()))?
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
            .map_err(|_e| Error::InternalServer(_e.to_string()))?;
        tokio::fs::write(path, &data)
            .await
            .map_err(|err| Error::InternalServer(err.to_string()))?;
    }

    Ok(filename)
}

pub async fn get_file_by_name(
    // PathWithAuthExtractor(filename): PathWithAuthExtractor<String>,
    PathExtractor(filename): PathExtractor<String>,
) -> Result<(HeaderMap, Vec<u8>), Error> {
    let bytes = std::fs::read(format!(
        "D:\\Projects\\rust\\im-backend\\upload/{}",
        filename
    ))
    .map_err(|err| Error::InternalServer(err.to_string()))?;
    let mut headers = HeaderMap::new();
    headers.insert(
        "Cache-Control",
        "private, max-age=31536000".parse().unwrap(),
    );
    Ok((headers, bytes))
}
