use utoipa::OpenApi;
use utoipa::ToSchema;

/// Main API documentation
/// 
/// This struct is used to generate the OpenAPI documentation for the entire API.
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::auth::register,
        crate::api::auth::login,
    ),
    components(
        schemas(
            crate::core::auth::RegisterRequest,
            crate::core::auth::LoginRequest,
            crate::api::auth::TokenResponse,
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints")
    )
)]
pub struct ApiDoc; 