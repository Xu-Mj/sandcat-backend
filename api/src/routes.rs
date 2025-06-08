use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, post, put};

use crate::AppState;
use crate::handlers::files::file::{get_avatar_by_name, get_file_by_name, upload, upload_avatar};
use crate::handlers::friends::friend_handlers::{
    agree, assign_friend_to_group, create_friend_group, create_friend_tag, create_friendship,
    delete_friend, delete_friend_group, delete_friend_tag, get_apply_list_by_user_id,
    get_friend_groups, get_friend_privacy, get_friend_tags, get_friends_list_by_user_id,
    get_interaction_stats, manage_friend_tags, query_friend_info, update_friend_group,
    update_friend_privacy, update_friend_remark, update_interaction_score,
};
use crate::handlers::groups::group_handlers::{
    change_member_role, close_poll, create_announcement, create_group_category,
    create_group_handler, create_poll, delete_announcement, delete_group_category,
    delete_group_file, delete_group_handler, get_group, get_group_and_members,
    get_group_announcements, get_group_categories, get_group_files, get_group_members,
    get_group_muted_members, get_group_polls, get_poll_details, invite_new_members,
    mute_group_member, pin_announcement, remove_member, unmute_group_member,
    update_file_pin_status, update_group_category, update_group_handler, update_group_settings,
    update_member_settings, upload_group_file, vote_poll,
};
use crate::handlers::messages::msg_handlers::{del_msg, get_seq, pull_offline_messages};
use crate::handlers::users::{
    create_user, get_user_by_id, github_callback, github_login, google_callback, google_login,
    login, logout, modify_pwd, refresh_token, search_user, send_email, update_user,
};

pub(crate) fn app_routes(state: AppState) -> Router {
    Router::new()
        .nest("/user", user_routes(state.clone()))
        .nest("/friend", friend_routes(state.clone()))
        .nest("/file", file_routes(state.clone()))
        .nest("/group", group_routes(state.clone()))
        .nest("/message", msg_routes(state.clone()))
}

fn friend_routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_friendship))
        .route("/:id/:offline_time", get(get_friends_list_by_user_id))
        .route("/:id/apply", get(get_apply_list_by_user_id))
        .route("/agree", put(agree))
        .route("/", delete(delete_friend))
        .route("/remark", put(update_friend_remark))
        .route("/query/:user_id", get(query_friend_info))
        .route("/groups", post(create_friend_group))
        .route("/groups", put(update_friend_group))
        .route("/groups/:group_id", delete(delete_friend_group))
        .route("/groups/:user_id", get(get_friend_groups))
        .route("/assign-group", post(assign_friend_to_group))
        .route("/tags", post(create_friend_tag))
        .route("/tags/:tag_id", delete(delete_friend_tag))
        .route("/tags/:user_id", get(get_friend_tags))
        .route("/manage-tags", post(manage_friend_tags))
        .route("/privacy", post(update_friend_privacy))
        .route("/privacy/:user_id/:friend_id", get(get_friend_privacy))
        .route("/interaction", post(update_interaction_score))
        .route(
            "/interaction/:user_id/:friend_id",
            get(get_interaction_stats),
        )
        .with_state(state)
}

fn user_routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_user))
        .route("/", put(update_user))
        .route("/pwd", put(modify_pwd))
        .route("/:id", get(get_user_by_id))
        .route("/refresh_token/:token/:is_refresh", get(refresh_token))
        .route("/:user_id/search/:pattern", get(search_user))
        .route("/login", post(login))
        .route("/logout/:uuid", delete(logout))
        .route("/mail/send", post(send_email))
        .route("/auth/wechat", get(google_login))
        .route("/auth/wechat/callback", get(google_callback))
        .route("/auth/github", get(github_login))
        .route("/auth/github/callback", get(github_callback))
        .with_state(state)
}

fn group_routes(state: AppState) -> Router {
    Router::new()
        .route("/:user_id/:group_id", get(get_group))
        .route("/:user_id", post(create_group_handler))
        .route("/invite", put(invite_new_members))
        .route("/", delete(delete_group_handler))
        .route("/:user_id", put(update_group_handler))
        .route("/member/:user_id/:group_id", get(get_group_and_members))
        .route("/member", post(get_group_members))
        .route("/member", delete(remove_member))
        // 群组设置相关
        .route("/settings", post(update_group_settings))
        .route("/member/settings", post(update_member_settings))
        // 群组分类相关
        .route("/categories", get(get_group_categories))
        .route("/category", post(create_group_category))
        .route("/category", put(update_group_category))
        .route("/category/:id", delete(delete_group_category))
        // 群组文件相关
        .route("/files/:group_id", get(get_group_files))
        .route("/file", post(upload_group_file))
        .route("/file", delete(delete_group_file))
        .route("/file/pin", post(update_file_pin_status))
        // 群组投票相关
        .route("/polls/:group_id", get(get_group_polls))
        .route("/poll/:poll_id", get(get_poll_details))
        .route("/poll", post(create_poll))
        .route("/poll/vote", post(vote_poll))
        .route("/poll/close", post(close_poll))
        // 群组禁言相关
        .route("/muted/:group_id", get(get_group_muted_members))
        .route("/mute", post(mute_group_member))
        .route("/unmute", post(unmute_group_member))
        // 群组公告相关
        .route("/announcements/:group_id", get(get_group_announcements))
        .route("/announcement", post(create_announcement))
        .route("/announcement", delete(delete_announcement))
        .route("/announcement/pin", post(pin_announcement))
        // 成员管理增强
        .route("/member/role", post(change_member_role))
        .with_state(state)
}

const MAX_FILE_UPLOAD_SIZE: usize = 1024 * 1024 * 50;
fn file_routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/upload",
            post(upload).layer(DefaultBodyLimit::max(MAX_FILE_UPLOAD_SIZE)),
        )
        .route("/get/:filename", get(get_file_by_name))
        .route(
            "/avatar/upload",
            post(upload_avatar).layer(DefaultBodyLimit::max(MAX_FILE_UPLOAD_SIZE)),
        )
        .route("/avatar/get/:filename", get(get_avatar_by_name))
        .with_state(state)
}

fn msg_routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(pull_offline_messages))
        .route("/seq/:user_id", get(get_seq))
        .route("/", delete(del_msg))
        .with_state(state)
}
