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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_can_be_created_with_owner_role() {
        let calendar = Calendar {
            id: "primary".to_string(),
            name: "My Calendar".to_string(),
            color: "#1a73e8".to_string(),
            is_primary: true,
            access_role: AccessRole::Owner,
        };

        assert_eq!(calendar.access_role, AccessRole::Owner);
    }

    #[test]
    fn calendar_can_be_created_with_writer_role() {
        let calendar = Calendar {
            id: "shared".to_string(),
            name: "Team Calendar".to_string(),
            color: "#e67c73".to_string(),
            is_primary: false,
            access_role: AccessRole::Writer,
        };

        assert_eq!(calendar.access_role, AccessRole::Writer);
    }

    #[test]
    fn calendar_can_be_created_with_reader_role() {
        let calendar = Calendar {
            id: "readonly".to_string(),
            name: "Public Calendar".to_string(),
            color: "#33b679".to_string(),
            is_primary: false,
            access_role: AccessRole::Reader,
        };

        assert_eq!(calendar.access_role, AccessRole::Reader);
    }
}
