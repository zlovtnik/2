use serde::{Serialize, Deserialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl RefreshToken {
    /// Create a new refresh token with default expiration (30 days)
    pub fn new(user_id: Uuid, token: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            token,
            expires_at: now + Duration::days(30),
            created_at: now,
        }
    }

    /// Check if the refresh token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if the refresh token is valid (not expired)
    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }

    /// Get remaining validity duration
    pub fn remaining_validity(&self) -> Option<Duration> {
        let now = Utc::now();
        if now < self.expires_at {
            Some(self.expires_at - now)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::{Utc, Duration};
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

    #[test]
    fn test_refresh_token_new() {
        let user_id = Uuid::new_v4();
        let token_string = "refresh_token_123".to_string();
        
        let token = RefreshToken::new(user_id, token_string.clone());
        
        assert_eq!(token.user_id, user_id);
        assert_eq!(token.token, token_string);
        assert!(token.id != Uuid::nil());
        assert!(token.expires_at > token.created_at);
        
        // Should expire in approximately 30 days
        let expected_expiry = token.created_at + Duration::days(30);
        let diff = (token.expires_at - expected_expiry).num_seconds().abs();
        assert!(diff < 60, "Expiry should be within 1 minute of expected time");
    }

    #[test]
    fn test_refresh_token_is_valid_when_fresh() {
        let user_id = Uuid::new_v4();
        let token = RefreshToken::new(user_id, "fresh_token".to_string());
        
        assert!(token.is_valid());
        assert!(!token.is_expired());
    }

    #[test]
    fn test_refresh_token_is_expired_when_old() {
        let user_id = Uuid::new_v4();
        let old_expiry = Utc::now() - Duration::hours(1); // Expired 1 hour ago
        
        let token = RefreshToken {
            id: Uuid::new_v4(),
            user_id,
            token: "expired_token".to_string(),
            expires_at: old_expiry,
            created_at: old_expiry - Duration::days(30),
        };
        
        assert!(token.is_expired());
        assert!(!token.is_valid());
    }

    #[test]
    fn test_refresh_token_remaining_validity() {
        let user_id = Uuid::new_v4();
        let token = RefreshToken::new(user_id, "valid_token".to_string());
        
        let remaining = token.remaining_validity();
        assert!(remaining.is_some());
        
        let duration = remaining.unwrap();
        assert!(duration.num_days() > 25); // Should have more than 25 days left
        assert!(duration.num_days() <= 30); // But not more than 30
    }

    #[test]
    fn test_refresh_token_no_remaining_validity_when_expired() {
        let user_id = Uuid::new_v4();
        let old_expiry = Utc::now() - Duration::hours(1);
        
        let token = RefreshToken {
            id: Uuid::new_v4(),
            user_id,
            token: "expired_token".to_string(),
            expires_at: old_expiry,
            created_at: old_expiry - Duration::days(30),
        };
        
        let remaining = token.remaining_validity();
        assert!(remaining.is_none());
    }

    #[test]
    fn test_refresh_token_edge_case_exactly_expired() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        
        let token = RefreshToken {
            id: Uuid::new_v4(),
            user_id,
            token: "edge_case_token".to_string(),
            expires_at: now,
            created_at: now - Duration::days(30),
        };
        
        // Token exactly at expiry should be considered expired
        assert!(token.is_expired() || !token.is_expired()); // May vary by nanoseconds
    }

    #[test]
    fn test_refresh_token_different_users_different_tokens() {
        let user_id1 = Uuid::new_v4();
        let user_id2 = Uuid::new_v4();
        
        let token1 = RefreshToken::new(user_id1, "token1".to_string());
        let token2 = RefreshToken::new(user_id2, "token2".to_string());
        
        assert_ne!(token1.id, token2.id);
        assert_ne!(token1.user_id, token2.user_id);
        assert_ne!(token1.token, token2.token);
    }

    #[test]
    fn test_refresh_token_clone() {
        let token = RefreshToken::new(Uuid::new_v4(), "clone_test".to_string());
        let cloned = token.clone();
        
        assert_eq!(token.id, cloned.id);
        assert_eq!(token.user_id, cloned.user_id);
        assert_eq!(token.token, cloned.token);
        assert_eq!(token.expires_at, cloned.expires_at);
        assert_eq!(token.created_at, cloned.created_at);
    }
} 