-- Migration: Create a simple PostgreSQL procedure that works with authenticated users
-- This procedure retrieves user information and can be called with the user ID from JWT token

CREATE OR REPLACE FUNCTION get_user_info_with_stats(p_user_id UUID)
RETURNS TABLE (
    user_id UUID,
    email TEXT,
    full_name TEXT,
    preferences JSONB,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ,
    refresh_token_count BIGINT,
    last_login TIMESTAMPTZ
) 
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
BEGIN
    -- Log the procedure call for audit purposes
    RAISE NOTICE 'get_user_info_with_stats called for user_id: %', p_user_id;
    
    -- Return user information along with some computed stats
    RETURN QUERY
    SELECT 
        u.id,
        u.email,
        u.full_name,
        u.preferences,
        u.created_at,
        u.updated_at,
        COALESCE(rt.token_count, 0) as refresh_token_count,
        rt.last_token_created as last_login
    FROM users u
    LEFT JOIN (
        SELECT 
            user_id,
            COUNT(*) as token_count,
            MAX(created_at) as last_token_created
        FROM refresh_tokens 
        WHERE user_id = p_user_id
        GROUP BY user_id
    ) rt ON u.id = rt.user_id
    WHERE u.id = p_user_id;
    
    -- If no user found, raise an exception
    IF NOT FOUND THEN
        RAISE EXCEPTION 'User with ID % not found', p_user_id;
    END IF;
END;
$$;

-- Grant execute permission to the application role (adjust as needed)
-- GRANT EXECUTE ON FUNCTION get_user_info_with_stats(UUID) TO your_app_role;