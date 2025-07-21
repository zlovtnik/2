use serde::{Serialize, Deserialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub preferences: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::Utc;
    use serde_json;

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            full_name: "Test User".to_string(),
            preferences: Some(serde_json::json!({"theme": "dark"})),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&user).expect("Should serialize");
        let deserialized: User = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(user.email, deserialized.email);
        assert_eq!(user.full_name, deserialized.full_name);
        assert_eq!(user.preferences, deserialized.preferences);
    }
} 