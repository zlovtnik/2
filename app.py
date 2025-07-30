import streamlit as st
import requests
import json
from datetime import datetime
import pandas as pd

# Configure Streamlit page
st.set_page_config(
    page_title="JWT Backend API Demo",
    page_icon="🔐",
    layout="wide",
    initial_sidebar_state="expanded"
)

# API Configuration
API_BASE_URL = st.sidebar.text_input(
    "API Base URL", 
    value="http://localhost:3000",
    help="Base URL of your Rust JWT backend API"
)

# Session state initialization
if 'access_token' not in st.session_state:
    st.session_state.access_token = None
if 'refresh_token' not in st.session_state:
    st.session_state.refresh_token = None
if 'user_data' not in st.session_state:
    st.session_state.user_data = None

def make_request(method, endpoint, data=None, auth_required=False):
    """Make HTTP request to the API"""
    url = f"{API_BASE_URL}{endpoint}"
    headers = {"Content-Type": "application/json"}
    
    if auth_required and st.session_state.access_token:
        headers["Authorization"] = f"Bearer {st.session_state.access_token}"
    
    try:
        if method == "GET":
            response = requests.get(url, headers=headers)
        elif method == "POST":
            response = requests.post(url, headers=headers, json=data)
        elif method == "PUT":
            response = requests.put(url, headers=headers, json=data)
        elif method == "DELETE":
            response = requests.delete(url, headers=headers)
        
        return response
    except requests.exceptions.RequestException as e:
        st.error(f"Request failed: {str(e)}")
        return None

def check_health():
    """Check API health status"""
    st.subheader("🏥 Health Check")
    
    col1, col2 = st.columns(2)
    
    with col1:
        if st.button("Check Live Status"):
            response = make_request("GET", "/health/live")
            if response and response.status_code == 200:
                st.success("✅ API is live!")
                st.json(response.json())
            else:
                st.error("❌ API is not responding")
    
    with col2:
        if st.button("Check Ready Status"):
            response = make_request("GET", "/health/ready")
            if response and response.status_code == 200:
                st.success("✅ API is ready!")
                st.json(response.json())
            else:
                st.error("❌ API is not ready")

def authentication_section():
    """Handle authentication operations"""
    st.subheader("🔐 Authentication")
    
    # Display current auth status
    if st.session_state.access_token:
        st.success("✅ Authenticated")
        if st.button("Logout"):
            st.session_state.access_token = None
            st.session_state.refresh_token = None
            st.session_state.user_data = None
            st.rerun()
    else:
        st.info("Not authenticated")
    
    tab1, tab2, tab3 = st.tabs(["Register", "Login", "Refresh Token"])
    
    with tab1:
        st.write("**Register New User**")
        with st.form("register_form"):
            email = st.text_input("Email")
            password = st.text_input("Password", type="password")
            name = st.text_input("Name")
            
            if st.form_submit_button("Register"):
                data = {
                    "email": email,
                    "password": password,
                    "name": name
                }
                response = make_request("POST", "/api/v1/auth/register", data)
                if response:
                    if response.status_code == 201:
                        st.success("✅ Registration successful!")
                        st.json(response.json())
                    else:
                        st.error(f"❌ Registration failed: {response.text}")
    
    with tab2:
        st.write("**Login**")
        with st.form("login_form"):
            email = st.text_input("Email", key="login_email")
            password = st.text_input("Password", type="password", key="login_password")
            
            if st.form_submit_button("Login"):
                data = {
                    "email": email,
                    "password": password
                }
                response = make_request("POST", "/api/v1/auth/login", data)
                if response:
                    if response.status_code == 200:
                        result = response.json()
                        st.session_state.access_token = result.get("access_token")
                        st.session_state.refresh_token = result.get("refresh_token")
                        st.success("✅ Login successful!")
                        st.rerun()
                    else:
                        st.error(f"❌ Login failed: {response.text}")
    
    with tab3:
        st.write("**Refresh Access Token**")
        if st.session_state.refresh_token:
            if st.button("Refresh Token"):
                data = {
                    "refresh_token": st.session_state.refresh_token
                }
                response = make_request("POST", "/api/v1/auth/refresh", data)
                if response:
                    if response.status_code == 200:
                        result = response.json()
                        st.session_state.access_token = result.get("access_token")
                        st.success("✅ Token refreshed!")
                    else:
                        st.error(f"❌ Token refresh failed: {response.text}")
        else:
            st.info("No refresh token available. Please login first.")

def user_management_section():
    """Handle user management operations"""
    st.subheader("👤 User Management")
    
    if not st.session_state.access_token:
        st.warning("Please login first to access user management features.")
        return
    
    tab1, tab2, tab3 = st.tabs(["Current User", "User Stats", "User Operations"])
    
    with tab1:
        st.write("**Current User Information**")
        if st.button("Get My Profile"):
            response = make_request("GET", "/api/v1/users/me", auth_required=True)
            if response:
                if response.status_code == 200:
                    user_data = response.json()
                    st.session_state.user_data = user_data
                    st.success("✅ Profile loaded!")
                    st.json(user_data)
                else:
                    st.error(f"❌ Failed to load profile: {response.text}")
    
    with tab2:
        st.write("**User Statistics**")
        if st.button("Get My Stats"):
            response = make_request("GET", "/api/v1/users/me/stats", auth_required=True)
            if response:
                if response.status_code == 200:
                    stats = response.json()
                    st.success("✅ Stats loaded!")
                    
                    # Display stats in a nice format
                    col1, col2, col3 = st.columns(3)
                    with col1:
                        st.metric("Total Logins", stats.get("total_logins", 0))
                    with col2:
                        st.metric("Last Login", stats.get("last_login", "Never"))
                    with col3:
                        st.metric("Account Created", stats.get("created_at", "Unknown"))
                    
                    st.json(stats)
                else:
                    st.error(f"❌ Failed to load stats: {response.text}")
    
    with tab3:
        st.write("**User Operations**")
        
        # Get specific user
        with st.expander("Get User by ID"):
            user_id = st.text_input("User ID")
            if st.button("Get User", key="get_user"):
                if user_id:
                    response = make_request("GET", f"/api/v1/users/{user_id}", auth_required=True)
                    if response:
                        if response.status_code == 200:
                            st.success("✅ User found!")
                            st.json(response.json())
                        else:
                            st.error(f"❌ User not found: {response.text}")
        
        # Update user
        with st.expander("Update User"):
            if st.session_state.user_data:
                user_id = st.session_state.user_data.get("id", "")
                st.text_input("User ID", value=user_id, disabled=True)
                
                with st.form("update_user_form"):
                    new_name = st.text_input("New Name", value=st.session_state.user_data.get("name", ""))
                    st.info("Note: Only the name can be updated via this endpoint")
                    
                    if st.form_submit_button("Update User"):
                        # The Rust API only accepts a name string, not a full object
                        response = make_request("PUT", f"/api/v1/users/{user_id}", new_name, auth_required=True)
                        if response:
                            if response.status_code == 200:
                                st.success("✅ User updated!")
                                st.json(response.json())
                                # Update session state with new data
                                st.session_state.user_data["name"] = new_name
                            else:
                                st.error(f"❌ Update failed: {response.text}")
            else:
                st.info("Load your profile first to update user information.")
        
        # Delete user
        with st.expander("Delete User", expanded=False):
            st.warning("⚠️ **Danger Zone** - This action cannot be undone!")
            user_id_to_delete = st.text_input("User ID to Delete", help="Enter the UUID of the user to delete")
            
            if st.button("🗑️ Delete User", type="secondary"):
                if user_id_to_delete:
                    if st.button("⚠️ Confirm Deletion", type="primary"):
                        response = make_request("DELETE", f"/api/v1/users/{user_id_to_delete}", auth_required=True)
                        if response:
                            if response.status_code == 204:
                                st.success("✅ User deleted successfully!")
                            elif response.status_code == 404:
                                st.error("❌ User not found")
                            else:
                                st.error(f"❌ Delete failed: {response.text}")
                else:
                    st.error("Please enter a User ID to delete")
        
        # Create user (separate from registration)
        with st.expander("Create User", expanded=False):
            st.write("**Create New User (Admin Function)**")
            with st.form("create_user_form"):
                email = st.text_input("Email", key="create_email")
                password = st.text_input("Password", type="password", key="create_password")
                name = st.text_input("Name", key="create_name")
                
                if st.form_submit_button("Create User"):
                    data = {
                        "email": email,
                        "password": password,
                        "name": name
                    }
                    response = make_request("POST", "/api/v1/users", data, auth_required=True)
                    if response:
                        if response.status_code == 201:
                            st.success("✅ User created successfully!")
                            st.json(response.json())
                        else:
                            st.error(f"❌ User creation failed: {response.text}")

def refresh_token_management_section():
    """Handle refresh token management operations"""
    st.subheader("🔄 Refresh Token Management")
    
    if not st.session_state.access_token:
        st.warning("Please login first to access refresh token management features.")
        return
    
    tab1, tab2, tab3, tab4 = st.tabs(["Create Token", "Get Token", "Update Token", "Delete Token"])
    
    with tab1:
        st.write("**Create New Refresh Token**")
        with st.form("create_refresh_token_form"):
            col1, col2 = st.columns(2)
            with col1:
                token_id = st.text_input("Token ID (UUID)", help="Leave empty to auto-generate")
                user_id = st.text_input("User ID (UUID)", help="User ID for this token")
            with col2:
                token_string = st.text_input("Token String", help="The actual refresh token string")
                expires_at = st.datetime_input("Expires At", help="When this token expires")
            
            if st.form_submit_button("Create Refresh Token"):
                from datetime import datetime
                import uuid
                
                # Generate UUID if not provided
                if not token_id:
                    token_id = str(uuid.uuid4())
                
                data = {
                    "id": token_id,
                    "user_id": user_id,
                    "token": token_string,
                    "expires_at": expires_at.isoformat() + "Z",
                    "created_at": datetime.utcnow().isoformat() + "Z"
                }
                
                response = make_request("POST", "/api/v1/refresh_tokens", data, auth_required=True)
                if response:
                    if response.status_code == 201:
                        st.success("✅ Refresh token created successfully!")
                        st.json(response.json())
                    elif response.status_code == 409:
                        st.error("❌ Refresh token with this ID already exists")
                    else:
                        st.error(f"❌ Creation failed: {response.text}")
    
    with tab2:
        st.write("**Get Refresh Token by ID**")
        with st.form("get_refresh_token_form"):
            token_id = st.text_input("Token ID (UUID)", key="get_token_id")
            
            if st.form_submit_button("Get Refresh Token"):
                if token_id:
                    response = make_request("GET", f"/api/v1/refresh_tokens/{token_id}", auth_required=True)
                    if response:
                        if response.status_code == 200:
                            token_data = response.json()
                            st.success("✅ Refresh token found!")
                            
                            # Display token info in a nice format
                            col1, col2 = st.columns(2)
                            with col1:
                                st.metric("Token ID", token_data.get("id", "N/A"))
                                st.metric("User ID", token_data.get("user_id", "N/A"))
                            with col2:
                                st.metric("Created At", token_data.get("created_at", "N/A"))
                                st.metric("Expires At", token_data.get("expires_at", "N/A"))
                            
                            st.json(token_data)
                        elif response.status_code == 404:
                            st.error("❌ Refresh token not found")
                        else:
                            st.error(f"❌ Failed to get token: {response.text}")
                else:
                    st.error("Please enter a Token ID")
    
    with tab3:
        st.write("**Update Refresh Token**")
        with st.form("update_refresh_token_form"):
            token_id = st.text_input("Token ID (UUID)", key="update_token_id")
            new_token_string = st.text_input("New Token String", key="new_token_string")
            
            if st.form_submit_button("Update Refresh Token"):
                if token_id and new_token_string:
                    # The Rust API expects just the new token string
                    response = make_request("PUT", f"/api/v1/refresh_tokens/{token_id}", new_token_string, auth_required=True)
                    if response:
                        if response.status_code == 200:
                            st.success("✅ Refresh token updated successfully!")
                            st.json(response.json())
                        elif response.status_code == 404:
                            st.error("❌ Refresh token not found")
                        else:
                            st.error(f"❌ Update failed: {response.text}")
                else:
                    st.error("Please enter both Token ID and new token string")
    
    with tab4:
        st.write("**Delete Refresh Token**")
        st.warning("⚠️ **Danger Zone** - This action cannot be undone!")
        
        with st.form("delete_refresh_token_form"):
            token_id = st.text_input("Token ID (UUID)", key="delete_token_id")
            confirm_delete = st.checkbox("I understand this action cannot be undone")
            
            if st.form_submit_button("🗑️ Delete Refresh Token", type="primary"):
                if token_id and confirm_delete:
                    response = make_request("DELETE", f"/api/v1/refresh_tokens/{token_id}", auth_required=True)
                    if response:
                        if response.status_code == 204:
                            st.success("✅ Refresh token deleted successfully!")
                        elif response.status_code == 404:
                            st.error("❌ Refresh token not found")
                        else:
                            st.error(f"❌ Delete failed: {response.text}")
                elif not token_id:
                    st.error("Please enter a Token ID")
                elif not confirm_delete:
                    st.error("Please confirm that you understand this action cannot be undone")

def api_testing_section():
    """Generic API testing interface"""
    st.subheader("🧪 API Testing")
    
    with st.form("api_test_form"):
        method = st.selectbox("HTTP Method", ["GET", "POST", "PUT", "DELETE"])
        endpoint = st.text_input("Endpoint", placeholder="/api/v1/users/me")
        
        # Request body for POST/PUT
        if method in ["POST", "PUT"]:
            request_body = st.text_area("Request Body (JSON)", placeholder='{"key": "value"}')
        else:
            request_body = None
        
        auth_required = st.checkbox("Requires Authentication")
        
        if st.form_submit_button("Send Request"):
            try:
                data = None
                if request_body:
                    data = json.loads(request_body)
                
                response = make_request(method, endpoint, data, auth_required)
                if response:
                    st.write(f"**Status Code:** {response.status_code}")
                    st.write("**Response Headers:**")
                    st.json(dict(response.headers))
                    st.write("**Response Body:**")
                    try:
                        st.json(response.json())
                    except:
                        st.text(response.text)
            except json.JSONDecodeError:
                st.error("Invalid JSON in request body")

def main():
    """Main application"""
    st.title("🔐 JWT Backend API Demo")
    st.markdown("---")
    
    # Sidebar navigation
    st.sidebar.title("Navigation")
    page = st.sidebar.selectbox(
        "Choose a section",
        ["Health Check", "Authentication", "User Management", "Refresh Token Management", "API Testing"]
    )
    
    # Display current authentication status in sidebar
    st.sidebar.markdown("---")
    st.sidebar.subheader("Auth Status")
    if st.session_state.access_token:
        st.sidebar.success("✅ Authenticated")
        if st.session_state.user_data:
            st.sidebar.write(f"**User:** {st.session_state.user_data.get('name', 'Unknown')}")
            st.sidebar.write(f"**Email:** {st.session_state.user_data.get('email', 'Unknown')}")
    else:
        st.sidebar.error("❌ Not Authenticated")
    
    # Main content based on selected page
    if page == "Health Check":
        check_health()
    elif page == "Authentication":
        authentication_section()
    elif page == "User Management":
        user_management_section()
    elif page == "Refresh Token Management":
        refresh_token_management_section()
    elif page == "API Testing":
        api_testing_section()
    
    # Footer
    st.markdown("---")
    st.markdown(
        """
        **About this app:**
        This Streamlit application provides a comprehensive web interface to interact with the Rust JWT Backend API.
        It demonstrates complete CRUD operations for users and refresh tokens, authentication flows, and API testing capabilities.
        
        **Features:**
        - **Health Check**: Monitor API live and ready status with database connectivity
        - **Authentication**: User registration, login, and JWT token refresh
        - **User Management**: Complete user CRUD operations
          - View current user profile and statistics
          - Get user by ID
          - Create new users (admin function)
          - Update user names
          - Delete users with confirmation
        - **Refresh Token Management**: Complete refresh token CRUD operations
          - Create new refresh tokens with auto-generated UUIDs
          - Retrieve refresh tokens by ID
          - Update refresh token strings
          - Delete refresh tokens with confirmation
        - **API Testing**: Generic interface for testing any API endpoint
        
        **API Endpoints Covered:**
        - `GET /health/live` & `GET /health/ready`
        - `POST /api/v1/auth/register`, `POST /api/v1/auth/login`, `POST /api/v1/auth/refresh`
        - `GET /api/v1/users/me`, `GET /api/v1/users/me/stats`, `GET /api/v1/users/{id}`
        - `POST /api/v1/users`, `PUT /api/v1/users/{id}`, `DELETE /api/v1/users/{id}`
        - `POST /api/v1/refresh_tokens`, `GET /api/v1/refresh_tokens/{id}`
        - `PUT /api/v1/refresh_tokens/{id}`, `DELETE /api/v1/refresh_tokens/{id}`
        """
    )

if __name__ == "__main__":
    main()