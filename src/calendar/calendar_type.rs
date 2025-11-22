use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String,
    pub name: String,
    pub color: String,
    pub is_primary: bool,
    pub access_role: AccessRole,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessRole {
    Owner,
    Writer,
    Reader,
}

