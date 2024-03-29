use crate::message::{
    GroupCreate, GroupCreateRequest, GroupDeleteRequest, GroupMembersIdRequest, GroupUpdate,
    GroupUpdateRequest,
};

impl GroupCreateRequest {
    pub fn new(group: GroupCreate) -> Self {
        Self { group: Some(group) }
    }
}

impl GroupUpdateRequest {
    pub fn new(group: GroupUpdate) -> Self {
        Self { group: Some(group) }
    }
}
impl GroupDeleteRequest {
    pub fn new(group_id: String, user_id: String) -> Self {
        Self { group_id, user_id }
    }
}
impl GroupMembersIdRequest {
    pub fn new(group_id: String) -> Self {
        Self { group_id }
    }
}
