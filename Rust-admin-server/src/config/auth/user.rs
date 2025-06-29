use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub role: String,
}

impl User {
    pub fn new(id: i64, username: String, password: String, role: String) -> Self {
        Self {
            id,
            username,
            password,
            role,
        }
    }

    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }

    pub fn is_user(&self) -> bool {
        self.role == "user"
    }
}
