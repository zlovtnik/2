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

impl User {
    /// Create a new user with default timestamps
    pub fn new(email: String, password_hash: String, full_name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            email,
            password_hash,
            full_name,
            preferences: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update user preferences
    pub fn update_preferences(&mut self, preferences: serde_json::Value) {
        self.preferences = Some(preferences);
        self.updated_at = Utc::now();
    }

    /// Check if email is valid format (basic validation)
    pub fn is_valid_email(email: &str) -> bool {
        if email.len() <= 5 {
            return false;
        }
        
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return false;
        }
        
        let local = parts[0];
        let domain = parts[1];
        
        // Local part cannot be empty
        if local.is_empty() {
            return false;
        }
        
        // Domain must contain a dot and have valid structure
        if !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.') {
            return false;
        }
        
        // Domain parts should not be empty
        let domain_parts: Vec<&str> = domain.split('.').collect();
        domain_parts.iter().all(|part| !part.is_empty())
    }

    /// Get user's display name (full_name or email if no full_name)
    pub fn display_name(&self) -> &str {
        if self.full_name.is_empty() {
            &self.email
        } else {
            &self.full_name
        }
    }
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

    #[test]
    fn test_user_new() {
        let email = "newuser@example.com".to_string();
        let password_hash = "hashedpassword123".to_string();
        let full_name = "New User".to_string();
        
        let user = User::new(email.clone(), password_hash.clone(), full_name.clone());
        
        assert_eq!(user.email, email);
        assert_eq!(user.password_hash, password_hash);
        assert_eq!(user.full_name, full_name);
        assert!(user.preferences.is_none());
        assert!(user.id != Uuid::nil());
        assert!(user.created_at <= Utc::now());
        assert_eq!(user.created_at, user.updated_at);
    }

    #[test]
    fn test_user_update_preferences() {
        let mut user = User::new(
            "pref@example.com".to_string(),
            "hash".to_string(),
            "Preference User".to_string(),
        );
        
        let original_updated_at = user.updated_at;
        
        // Small delay to ensure updated_at changes
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        let preferences = serde_json::json!({
            "theme": "light",
            "notifications": true,
            "language": "en"
        });
        
        user.update_preferences(preferences.clone());
        
        assert_eq!(user.preferences, Some(preferences));
        assert!(user.updated_at > original_updated_at);
    }

    #[test]
    fn test_user_valid_email() {
        assert!(User::is_valid_email("user@example.com"));
        assert!(User::is_valid_email("test.email@domain.co.uk"));
        assert!(User::is_valid_email("simple@test.org"));
    }

    #[test]
    fn test_user_invalid_email() {
        assert!(!User::is_valid_email(""));
        assert!(!User::is_valid_email("invalid"));
        assert!(!User::is_valid_email("@example.com"));
        assert!(!User::is_valid_email("user@"));
        assert!(!User::is_valid_email("user@.com"));
        assert!(!User::is_valid_email("user.example.com"));
        assert!(!User::is_valid_email("a@b"));
    }

    #[test]
    fn test_user_display_name_with_full_name() {
        let user = User::new(
            "display@example.com".to_string(),
            "hash".to_string(),
            "Display Name".to_string(),
        );
        
        assert_eq!(user.display_name(), "Display Name");
    }

    #[test]
    fn test_user_display_name_without_full_name() {
        let user = User::new(
            "fallback@example.com".to_string(),
            "hash".to_string(),
            "".to_string(),
        );
        
        assert_eq!(user.display_name(), "fallback@example.com");
    }

    #[test]
    fn test_user_clone() {
        let user = User::new(
            "clone@example.com".to_string(),
            "clonehash".to_string(),
            "Clone User".to_string(),
        );
        
        let cloned = user.clone();
        
        assert_eq!(user.id, cloned.id);
        assert_eq!(user.email, cloned.email);
        assert_eq!(user.password_hash, cloned.password_hash);
        assert_eq!(user.full_name, cloned.full_name);
        assert_eq!(user.preferences, cloned.preferences);
        assert_eq!(user.created_at, cloned.created_at);
        assert_eq!(user.updated_at, cloned.updated_at);
    }

    #[test]
    fn test_user_preferences_json_roundtrip() {
        let mut user = User::new(
            "json@example.com".to_string(),
            "hash".to_string(),
            "JSON User".to_string(),
        );
        
        let complex_preferences = serde_json::json!({
            "ui": {
                "theme": "dark",
                "sidebar_collapsed": false,
                "notifications": {
                    "email": true,
                    "push": false,
                    "desktop": true
                }
            },
            "features": ["advanced_search", "export_data"],
            "version": 2
        });
        
        user.update_preferences(complex_preferences.clone());
        
        // Serialize the entire user
        let json = serde_json::to_string(&user).expect("Should serialize user");
        let deserialized_user: User = serde_json::from_str(&json).expect("Should deserialize user");
        
        assert_eq!(user.preferences, deserialized_user.preferences);
        assert_eq!(complex_preferences, deserialized_user.preferences.unwrap());
    }

    #[test]
    fn test_user_multiple_preference_updates() {
        let mut user = User::new(
            "multiple@example.com".to_string(),
            "hash".to_string(),
            "Multiple Updates".to_string(),
        );
        
        let pref1 = serde_json::json!({"theme": "light"});
        user.update_preferences(pref1.clone());
        let first_update = user.updated_at;
        
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        let pref2 = serde_json::json!({"theme": "dark", "lang": "es"});
        user.update_preferences(pref2.clone());
        
        assert_eq!(user.preferences, Some(pref2));
        assert!(user.updated_at > first_update);
    }
} 