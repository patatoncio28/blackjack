use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum VaultOp {
    Set,
    Add,
    Subtract,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MutateRequest {
    pub user_id: String,
    pub game_id: String,
    pub key: String,
    pub op: VaultOp,
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MutateResponse {
    pub success: bool,
    pub key: String,
    pub new_value: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReadRequest {
    pub user_id: String,
    pub game_id: String,
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReadResponse {
    pub success: bool,
    pub key: String,
    pub value: Option<Value>,
}

// POSIX-like permissions (Read=4, Write=2, Execute=1)
// E.g. Owner (User), Group (Game/Platform), Others
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct PosixPermissions {
    pub owner: u8, // Owner user permissions (e.g. 6 = RW)
    pub group: u8, // Group (Game) permissions (e.g. 4 = R)
    pub other: u8, // Others permissions (e.g. 0)
}

impl Default for PosixPermissions {
    fn default() -> Self {
        Self {
            owner: 6, // Read & Write
            group: 4, // Read only
            other: 0, // No access
        }
    }
}
