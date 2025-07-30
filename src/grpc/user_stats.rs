use tonic::{Request, Response, Status};
use sqlx::PgPool;
use uuid::Uuid;
use tracing::{info, warn, error, debug};
use crate::core::auth::verify_jwt;
use crate::api::user::UserInfoWithStats;

// Include the generated protobuf code
pub mod user_stats {
    tonic::include_proto!("user_stats");
}

use user_stats::{
    user_stats_service_server::UserStatsService,
    GetCurrentUserStatsRequest,
    GetCurrentUserStatsResponse,
};

pub struct UserStatsServiceImpl {
    pool: PgPool,
}

impl UserStatsServiceImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Extract JWT token from gRPC metadata and verify it
    fn extract_and_verify_token(request: &Request<GetCurrentUserStatsRequest>) -> Result<Uuid, Status> {
        debug!("Extracting JWT token from gRPC metadata");
        
        // Get the authorization header from metadata
        let auth_header = request
            .metadata()
            .get("authorization")
            .ok_or_else(|| {
                warn!("Missing authorization header in gRPC request");
                Status::unauthenticated("Missing authorization header")
            })?;

        // Convert MetadataValue to string
        let auth_str = auth_header
            .to_str()
            .map_err(|e| {
                error!(error = %e, "Invalid authorization header format");
                Status::unauthenticated("Invalid authorization header format")
            })?;

        // Extract Bearer token
        let token = auth_str
            .strip_prefix("Bearer ")
            .ok_or_else(|| {
                warn!("Authorization header missing Bearer prefix");
                Status::unauthenticated("Invalid authorization header format")
            })?;

        debug!("JWT token extracted, verifying...");
        
        // Verify JWT token and extract user ID
        verify_jwt(token).map_err(|e| {
            error!(error = %e, "JWT token verification failed");
            Status::unauthenticated("Invalid or expired token")
        })
    }
}

#[tonic::async_trait]
impl UserStatsService for UserStatsServiceImpl {
    async fn get_current_user_stats(
        &self,
        request: Request<GetCurrentUserStatsRequest>,
    ) -> Result<Response<GetCurrentUserStatsResponse>, Status> {
        let start_time = std::time::Instant::now();
        info!("gRPC GetCurrentUserStats called");
        
        // Extract and verify JWT token
        let user_id = Self::extract_and_verify_token(&request)?;
        
        info!(user_id = %user_id, "Getting current user stats via gRPC and PostgreSQL procedure");
        debug!("Calling get_user_info_with_stats procedure via gRPC");
        
        // Call the PostgreSQL procedure with the authenticated user's ID
        let query = "SELECT * FROM get_user_info_with_stats($1)";
        
        match sqlx::query_as::<_, UserInfoWithStats>(query)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
        {
            Ok(user_stats) => {
                info!(
                    user_id = %user_id, 
                    email = %user_stats.email,
                    refresh_token_count = user_stats.refresh_token_count,
                    "User stats retrieved successfully via gRPC procedure"
                );
                debug!(
                    full_name = %user_stats.full_name,
                    created_at = %user_stats.created_at,
                    last_login = ?user_stats.last_login,
                    "Detailed user stats from gRPC procedure"
                );

                // Convert to gRPC response
                let response = GetCurrentUserStatsResponse {
                    user_id: user_stats.user_id.to_string(),
                    email: user_stats.email,
                    full_name: user_stats.full_name,
                    preferences: user_stats.preferences.map(|p| p.to_string()),
                    created_at: user_stats.created_at.to_rfc3339(),
                    updated_at: user_stats.updated_at.to_rfc3339(),
                    refresh_token_count: user_stats.refresh_token_count,
                    last_login: user_stats.last_login.map(|dt| dt.to_rfc3339()),
                };

                let duration = start_time.elapsed();
                info!(
                    user_id = %user_id,
                    duration_ms = duration.as_millis(),
                    "gRPC GetCurrentUserStats completed successfully"
                );

                Ok(Response::new(response))
            },
            Err(e) => {
                let duration = start_time.elapsed();
                error!(
                    user_id = %user_id, 
                    error = %e, 
                    duration_ms = duration.as_millis(),
                    "Failed to retrieve user stats via gRPC procedure"
                );
                if e.to_string().contains("not found") {
                    warn!(user_id = %user_id, "User not found in gRPC procedure call");
                    Err(Status::not_found("User not found"))
                } else {
                    Err(Status::internal(format!("Database error: {}", e)))
                }
            },
        }
    }
}