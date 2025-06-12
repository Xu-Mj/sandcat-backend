use std::process::Command;

pub trait BuilderExt {
    fn with_sqlx_type(self, path: &[&str]) -> Self;
    fn with_derive_builder(self, path: &[&str]) -> Self;
    fn with_derive_builder_into(self, path: &str, attr: &[&str]) -> Self;
    fn with_derive_builder_option(self, path: &str, attr: &[&str]) -> Self;
    fn with_serde(self, path: &[&str]) -> Self;
}

impl BuilderExt for tonic_build::Builder {
    // set sqlx::Type for ReservationStatus
    fn with_sqlx_type(self, path: &[&str]) -> Self {
        // fold func: do somethin with given closure to given initial value; return final value
        path.iter().fold(self, |acc, path| {
            acc.type_attribute(path, "#[derive(sqlx::Type)]")
        })
    }

    fn with_derive_builder(self, path: &[&str]) -> Self {
        path.iter().fold(self, |acc, path| {
            acc.type_attribute(path, "#[derive(derive_builder::Builder)]")
        })
    }

    fn with_derive_builder_into(self, path: &str, field: &[&str]) -> Self {
        field.iter().fold(self, |acc, field| {
            acc.field_attribute(
                format!("{path}.{field}"),
                "#[builder(setter(into), default)]",
            )
        })
    }

    fn with_derive_builder_option(self, path: &str, field: &[&str]) -> Self {
        field.iter().fold(self, |acc, field| {
            acc.field_attribute(
                format!("{path}.{field}"),
                "#[builder(setter(strip_option, into), default)]",
            )
        })
    }

    fn with_serde(self, path: &[&str]) -> Self {
        path.iter().fold(self, |acc, path| {
            acc.type_attribute(path, "#[derive(serde::Serialize, serde::Deserialize)]")
                .type_attribute(path, "#[serde(rename_all = \"camelCase\")]")
        })
    }
}
fn main() {
    tonic_build::configure()
        .out_dir("src/pb")
        .field_attribute("User.password", "#[serde(skip_serializing)]")
        .field_attribute("User.salt", "#[serde(skip_serializing)]")
        .with_serde(&[
            // Common types
            "PlatformType",
            "ContentType",
            "FriendshipStatus",
            "MsgType",
            "SingleCallInviteType",
            "GroupMemberRole",
            "Msg",
            "MsgContent",
            "Mention",
            "MsgRead",
            "Candidate",
            "AgreeSingleCall",
            "SingleCallInvite",
            "SingleCallInviteAnswer",
            "SingleCallInviteNotAnswer",
            "SingleCallInviteCancel",
            "SingleCallOffer",
            "Hangup",
            "Single",
            "MsgResponse",
            // Client types
            "GroupInfo",
            "GroupMember",
            "GroupInvitation",
            "GroupInviteNew",
            "GroupUpdate",
            "User",
            "UserWithMatchType",
            "Friendship",
            "FriendshipWithUser",
            "Friend",
            "FriendInfo",
            "FsCreate",
            "AgreeReply",
            "FsUpdate",
            "GetGroupAndMembersResp",
            "PullOfflineRequest",
            "PullOfflineResponse",
            // Server types
            "UserAndGroupId",
            "GroupCategory",
            "GroupFile",
            "PollOption",
            "GroupPoll",
            "GroupMuteRecord",
            "GroupAnnouncement",
            "CreateGroupCategoryRequest",
            "UpdateGroupRequest",
            "GroupFileUploadRequest",
            "CreatePollRequest",
            "VotePollRequest",
            "MuteGroupMemberRequest",
            "CreateAnnouncementRequest",
            "GroupCreate",
            "FriendGroup",
            "UpdateFriendGroupRequest",
            "FriendTag",
            "ManageFriendTagRequest",
            "FriendPrivacySettings",
            "UpdateFriendPrivacyRequest",
            "UpdateRemarkRequest",
            "DeleteFriendRequest",
            "RemoveMemberRequest",
            "GroupMembersIdRequest",
            "SendMsgRequest",
            "SendGroupMsgRequest",
            "SaveMessageRequest",
            "SaveGroupMsgRequest",
            "GroupMemSeq",
            "GetDbMsgRequest",
            "GetDbMessagesRequest",
            "DelMsgRequest",
            "GetMemberReq",
            "MsgReadReq",
            "MsgReadResp",
            "GroupInviteNewRequest",
            "RemoveMemberResp",
            "GroupInviteNewResp",
            "GroupUpdateRequest",
            "GroupUpdateResponse",
            "GroupDeleteRequest",
            "GroupDeleteResponse",
            "GroupMemberExitResponse",
            "GroupMembersIdResponse",
            "CreateUserRequest",
            "CreateUserResponse",
            "GetUserRequest",
            "GetUserByEmailRequest",
            "GetUserResponse",
            "UpdateUserRequest",
            "UpdateUserResponse",
            "UpdateRegionRequest",
            "UpdateRegionResponse",
            "SearchUserRequest",
            "SearchUserResponse",
            "FsCreateRequest",
            "FsCreateResponse",
            "FsAgreeRequest",
            "FsAgreeResponse",
            "UpdateRemarkResponse",
            "DeleteFriendResponse",
            "FsUpdateRequest",
            "FriendListRequest",
            "FsListResponse",
            "FriendListResponse",
            "ClosePollRequest",
            "UpdateFilePinRequest",
            "UnmuteRequest",
            "UserUpdate",
            "SendMsgResponse",
        ])
        .with_sqlx_type(&["FriendshipStatus", "GroupMemberRole"])
        .compile(
            &[
                "protos/common.proto",
                "protos/client_messages.proto",
                "protos/server_messages.proto",
            ],
            &["protos"],
        )
        .unwrap();

    // execute cargo fmt command
    Command::new("cargo").arg("fmt").output().unwrap();

    println!("cargo: rerun-if-changed=abi/protos/common.proto");
    println!("cargo: rerun-if-changed=abi/protos/client_messages.proto");
    println!("cargo: rerun-if-changed=abi/protos/server_messages.proto");
}
