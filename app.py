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
                    new_email = st.text_input("New Email", value=st.session_state.user_data.get("email", ""))
                    
                    if st.form_submit_button("Update User"):
                        data = {
                            "name": new_name,
                            "email": new_email
                        }
                        response = make_request("PUT", f"/api/v1/users/{user_id}", data, auth_required=True)
                        if response:
                            if response.status_code == 200:
                                st.success("✅ User updated!")
                                st.json(response.json())
                            else:
                                st.error(f"❌ Update failed: {response.text}")
            else:
                st.info("Load your profile first to update user information.")

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
        ["Health Check", "Authentication", "User Management", "API Testing"]
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
    elif page == "API Testing":
        api_testing_section()
    
    # Footer
    st.markdown("---")
    st.markdown(
        """
        **About this app:**
        This Streamlit application provides a web interface to interact with the Rust JWT Backend API.
        It demonstrates authentication, user management, and general API testing capabilities.
        
        **Features:**
        - Health check monitoring
        - User registration and authentication
        - JWT token management
        - User profile management
        - Generic API testing interface
        """
    )

if __name__ == "__main__":
    main()