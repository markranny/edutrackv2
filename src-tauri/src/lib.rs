use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use tauri::State;
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: Option<i32>,
    email: String,
    password: String,
    role: String,
    firstname: Option<String>,
    lastname: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
    role: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SignupRequest {
    email: String,
    password: String,
    role: String,
    firstname: Option<String>,
    lastname: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginResponse {
    success: bool,
    message: String,
    user: Option<User>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChangePasswordRequest {
    email: String,
    current_password: String,
    new_password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserProfile {
    email: String,
    firstname: Option<String>,
    lastname: Option<String>,
    role: String,
}

// Database connection wrapper
struct DbConnection(Mutex<Connection>);

// Initialize database with proper migration
fn init_database() -> Result<Connection> {
    let conn = Connection::open("my_database.db")?;
    
    // Create users table (original schema)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL,
            role TEXT NOT NULL
        )",
        [],
    )?;

    // Check if firstname and lastname columns exist, if not add them
    let mut has_firstname = false;
    let mut has_lastname = false;

    // Check existing columns
    {
        let mut stmt = conn.prepare("PRAGMA table_info(users)")?;
        let column_info = stmt.query_map([], |row| {
            Ok(row.get::<_, String>(1)?) // Column name is at index 1
        })?;

        for column_result in column_info {
            if let Ok(column_name) = column_result {
                if column_name == "firstname" {
                    has_firstname = true;
                }
                if column_name == "lastname" {
                    has_lastname = true;
                }
            }
        }
    } // stmt is dropped here

    // Add missing columns
    if !has_firstname {
        conn.execute("ALTER TABLE users ADD COLUMN firstname TEXT", [])?;
    }
    if !has_lastname {
        conn.execute("ALTER TABLE users ADD COLUMN lastname TEXT", [])?;
    }

    Ok(conn)
}

#[tauri::command]
async fn tauri_login(payload: LoginRequest, db: State<'_, DbConnection>) -> Result<LoginResponse, String> {
    let conn = db.0.lock().map_err(|_| "Database lock error")?;
    
    // Query user from database including firstname and lastname
    let mut stmt = conn.prepare("SELECT id, email, password, role, firstname, lastname FROM users WHERE email = ? AND role = ?")
        .map_err(|e| format!("Database error: {}", e))?;
    
    let user_result = stmt.query_row([&payload.email, &payload.role], |row| {
        Ok(User {
            id: Some(row.get(0)?),
            email: row.get(1)?,
            password: row.get(2)?,
            role: row.get(3)?,
            firstname: row.get::<_, Option<String>>(4)?,
            lastname: row.get::<_, Option<String>>(5)?,
        })
    });

    match user_result {
        Ok(user) => {
            // Verify password
            if bcrypt::verify(&payload.password, &user.password).unwrap_or(false) {
                Ok(LoginResponse {
                    success: true,
                    message: "Login successful".to_string(),
                    user: Some(User {
                        id: user.id,
                        email: user.email,
                        password: "".to_string(), // Don't return password
                        role: user.role,
                        firstname: user.firstname,
                        lastname: user.lastname,
                    }),
                })
            } else {
                Ok(LoginResponse {
                    success: false,
                    message: "Invalid credentials".to_string(),
                    user: None,
                })
            }
        }
        Err(_) => Ok(LoginResponse {
            success: false,
            message: "User not found".to_string(),
            user: None,
        }),
    }
}

#[tauri::command]
async fn tauri_register(payload: SignupRequest, db: State<'_, DbConnection>) -> Result<String, String> {
    let conn = db.0.lock().map_err(|_| "Database lock error")?;
    
    // Hash password
    let hashed_password = bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST)
        .map_err(|_| "Password hashing failed")?;

    // Insert user into database with firstname and lastname
    let firstname_val = payload.firstname.unwrap_or_default();
    let lastname_val = payload.lastname.unwrap_or_default();
    
    let result = conn.execute(
        "INSERT INTO users (email, password, role, firstname, lastname) VALUES (?1, ?2, ?3, ?4, ?5)",
        [
            &payload.email, 
            &hashed_password, 
            &payload.role,
            &firstname_val,
            &lastname_val
        ],
    );

    match result {
        Ok(_) => Ok("Registration successful".to_string()),
        Err(rusqlite::Error::SqliteFailure(err, _)) => {
            if err.code == rusqlite::ErrorCode::ConstraintViolation {
                Err("Email already exists".to_string())
            } else {
                Err("Database error occurred".to_string())
            }
        }
        Err(_) => Err("Registration failed".to_string()),
    }
}

#[tauri::command]
async fn get_user_profile(email: String, db: State<'_, DbConnection>) -> Result<UserProfile, String> {
    let conn = db.0.lock().map_err(|_| "Database lock error")?;
    
    let mut stmt = conn.prepare("SELECT email, firstname, lastname, role FROM users WHERE email = ?")
        .map_err(|e| format!("Database error: {}", e))?;
    
    let profile_result = stmt.query_row([&email], |row| {
        Ok(UserProfile {
            email: row.get(0)?,
            firstname: row.get::<_, Option<String>>(1)?,
            lastname: row.get::<_, Option<String>>(2)?,
            role: row.get(3)?,
        })
    });

    match profile_result {
        Ok(profile) => Ok(profile),
        Err(_) => Err("User not found".to_string()),
    }
}

#[tauri::command]
async fn change_password(payload: ChangePasswordRequest, db: State<'_, DbConnection>) -> Result<String, String> {
    let conn = db.0.lock().map_err(|_| "Database lock error")?;
    
    // First, verify the current password
    let mut stmt = conn.prepare("SELECT password FROM users WHERE email = ?")
        .map_err(|e| format!("Database error: {}", e))?;
    
    let current_hash_result = stmt.query_row([&payload.email], |row| {
        Ok(row.get::<_, String>(0)?)
    });

    match current_hash_result {
        Ok(current_hash) => {
            // Verify current password
            if !bcrypt::verify(&payload.current_password, &current_hash).unwrap_or(false) {
                return Err("Current password is incorrect".to_string());
            }

            // Hash new password
            let new_hashed_password = bcrypt::hash(&payload.new_password, bcrypt::DEFAULT_COST)
                .map_err(|_| "Password hashing failed")?;

            // Update password
            let result = conn.execute(
                "UPDATE users SET password = ? WHERE email = ?",
                [&new_hashed_password, &payload.email],
            );

            match result {
                Ok(updated) => {
                    if updated > 0 {
                        Ok("Password changed successfully".to_string())
                    } else {
                        Err("User not found".to_string())
                    }
                }
                Err(_) => Err("Password change failed".to_string()),
            }
        }
        Err(_) => Err("User not found".to_string()),
    }
}

#[tauri::command]
async fn tauri_forgot_password(email: String, db: State<'_, DbConnection>) -> Result<String, String> {
    let conn = db.0.lock().map_err(|_| "Database lock error")?;
    
    // Check if user exists
    let mut stmt = conn.prepare("SELECT id FROM users WHERE email = ?")
        .map_err(|e| format!("Database error: {}", e))?;
    
    let user_exists = stmt.exists([&email])
        .map_err(|e| format!("Database error: {}", e))?;

    if user_exists {
        // Generate a simple reset token (in production, use proper token generation)
        let token = format!("RESET_{}", chrono::Utc::now().timestamp());
        Ok(format!("Reset token: {}", token))
    } else {
        Err("Email not found".to_string())
    }
}

#[tauri::command]
async fn reset_password(email: String, _token: String, new_password: String, db: State<'_, DbConnection>) -> Result<String, String> {
    let conn = db.0.lock().map_err(|_| "Database lock error")?;
    
    // In a real app, you'd verify the token here
    // For now, we'll just check if user exists and update password
    
    let hashed_password = bcrypt::hash(&new_password, bcrypt::DEFAULT_COST)
        .map_err(|_| "Password hashing failed")?;

    let result = conn.execute(
        "UPDATE users SET password = ? WHERE email = ?",
        [&hashed_password, &email],
    );

    match result {
        Ok(updated) => {
            if updated > 0 {
                Ok("Password reset successful".to_string())
            } else {
                Err("User not found".to_string())
            }
        }
        Err(_) => Err("Password reset failed".to_string()),
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize database
    let conn = init_database().expect("Failed to initialize database");
    let db_state = DbConnection(Mutex::new(conn));

    tauri::Builder::default()
        .manage(db_state)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            tauri_login,
            tauri_register,
            tauri_forgot_password,
            reset_password,
            get_user_profile,
            change_password
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}