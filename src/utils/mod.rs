pub(crate) use custom_extract::auth::JsonWithAuthExtractor;
pub(crate) use custom_extract::auth::PathWithAuthExtractor;
pub(crate) use custom_extract::json_extractor::JsonExtractor;
pub(crate) use custom_extract::path_extractor::PathExtractor;

mod custom_extract;
pub(crate) mod redis;

/// 格式化时间
pub fn format_milliseconds(millis: i64) -> String {
    let duration = chrono::Duration::milliseconds(millis);

    let seconds = duration.num_seconds();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}
