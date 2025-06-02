use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::State;
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: Option<i32>,
    email: String,
    password: String,
    role: String,
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

// Database connection wrapper
struct DbConnection(Mutex<Connection>);

// Initialize database
fn init_database() -> Result<Connection> {
    let conn = Connection::open("my_database.db")?;
    
    // Create users table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL,
            role TEXT NOT NULL
        )",
        [],
    )?;

    Ok(conn)
}

#[tauri::command]
async fn tauri_login(payload: LoginRequest, db: State<'_, DbConnection>) -> Result<LoginResponse, String> {
    let conn = db.0.lock().map_err(|_| "Database lock error")?;
    
    // Query user from database
    let mut stmt = conn.prepare("SELECT id, email, password, role FROM users WHERE email = ? AND role = ?")
        .map_err(|e| format!("Database error: {}", e))?;
    
    let user_result = stmt.query_row([&payload.email, &payload.role], |row| {
        Ok(User {
            id: Some(row.get(0)?),
            email: row.get(1)?,
            password: row.get(2)?,
            role: row.get(3)?,
        })
    });

    match user_result {
        Ok(user) => {
            // Verify password (you should use bcrypt in production)
            if bcrypt::verify(&payload.password, &user.password).unwrap_or(false) {
                Ok(LoginResponse {
                    success: true,
                    message: "Login successful".to_string(),
                    user: Some(User {
                        id: user.id,
                        email: user.email,
                        password: "".to_string(), // Don't return password
                        role: user.role,
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

    // Insert user into database
    let result = conn.execute(
        "INSERT INTO users (email, password, role) VALUES (?1, ?2, ?3)",
        [&payload.email, &hashed_password, &payload.role],
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
async fn reset_password(email: String, token: String, new_password: String, db: State<'_, DbConnection>) -> Result<String, String> {
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
            reset_password
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}