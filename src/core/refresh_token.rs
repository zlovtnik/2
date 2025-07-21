use serde::{Serialize, Deserialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::Utc;
    use serde_json;

    #[test]
    fn test_refresh_token_serialization() {
        let token = RefreshToken {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            token: "sometokenstring".to_string(),
            expires_at: Utc::now(),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&token).expect("Should serialize");
        let deserialized: RefreshToken = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(token.token, deserialized.token);
        assert_eq!(token.user_id, deserialized.user_id);
        assert_eq!(token.expires_at.timestamp(), deserialized.expires_at.timestamp());
    }
} 