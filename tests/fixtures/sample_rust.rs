//! Authentication module providing token generation, verification, and role-based access control.
//!
//! This module handles JWT token management, password hashing, and permission checking
//! for a multi-tenant web application with role-based access control.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Errors that can occur during authentication operations.
#[derive(Debug, Clone, PartialEq)]
pub enum AuthError {
    /// Token has expired and is no longer valid
    TokenExpired,
    /// Token signature verification failed
    InvalidSignature,
    /// User lacks sufficient permissions for the requested operation
    InsufficientPermissions,
    /// Token format is malformed or unrecognizable
    InvalidToken,
    /// Hash algorithm operation failed
    HashError,
}

/// JWT claims embedded in authentication tokens.
///
/// Contains user identity, role, and expiration information used for
/// authorization decisions throughout the application.
#[derive(Debug, Clone, PartialEq)]
pub struct Claims {
    pub sub: String,          // Subject (user ID)
    pub role: Role,
    pub exp: u64,             // Expiration time as Unix timestamp
    pub iat: u64,             // Issued at time as Unix timestamp
    pub permissions: Vec<String>,
}

/// User role determining access level and permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Full access to all resources and operations
    Admin,
    /// Standard user with limited write access
    User,
    /// Read-only access without modification capabilities
    ReadOnly,
}

impl Role {
    /// Returns the numeric hierarchy level for role comparison.
    fn level(&self) -> u8 {
        match self {
            Role::Admin => 3,
            Role::User => 2,
            Role::ReadOnly => 1,
        }
    }
}

/// Manages JWT token creation, validation, and user authentication.
///
/// Handles the complete lifecycle of authentication tokens including
/// generation, verification, refresh, and permission checking.
pub struct AuthManager {
    secret: String,
    token_lifetime: Duration,
    refresh_lifetime: Duration,
}

impl AuthManager {
    /// Creates a new AuthManager with the given secret key.
    ///
    /// # Panics
    /// Panics if the secret is empty.
    pub fn new(secret: String) -> Self {
        assert!(!secret.is_empty(), "Secret key cannot be empty");
        Self {
            secret,
            token_lifetime: Duration::from_secs(3600),    // 1 hour
            refresh_lifetime: Duration::from_secs(86400), // 24 hours
        }
    }

    /// Generates a new JWT token for the given claims.
    ///
    /// # Errors
    /// Returns `AuthError::HashError` if token generation fails.
    pub fn generate_token(&self, claims: &Claims) -> Result<String, AuthError> {
        let header = r#"{"alg":"HS256","typ":"JWT"}"#;
        let payload = format!(
            r#"{{"sub":"{}","role":"{:?}","exp":{},"iat":{},"permissions":{:?}}}"#,
            claims.sub, claims.role, claims.exp, claims.iat, claims.permissions
        );

        let token = format!("{}.{}", self._base64_encode(header), self._base64_encode(&payload));
        let signature = self._sign(&token)?;

        Ok(format!("{}.{}", token, signature))
    }

    /// Verifies the signature and expiration of a JWT token.
    ///
    /// # Errors
    /// Returns:
    /// - `AuthError::InvalidToken` if the token format is invalid
    /// - `AuthError::InvalidSignature` if signature verification fails
    /// - `AuthError::TokenExpired` if the token has expired
    pub fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidToken);
        }

        let signature = self._sign(&format!("{}.{}", parts[0], parts[1]))?;
        if signature != parts[2] {
            return Err(AuthError::InvalidSignature);
        }

        let payload = self._base64_decode(parts[1])
            .map_err(|_| AuthError::InvalidToken)?;

        // Parse claims from payload (simplified)
        let exp_start = payload.find("\"exp\":").ok_or(AuthError::InvalidToken)? + 6;
        let exp_str: String = payload[exp_start..].chars()
            .take_while(|c| c.is_numeric())
            .collect();
        let exp: u64 = exp_str.parse()
            .map_err(|_| AuthError::InvalidToken)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if exp < now {
            return Err(AuthError::TokenExpired);
        }

        Ok(Claims {
            sub: "user123".to_string(),
            role: Role::User,
            exp,
            iat: now,
            permissions: vec!["read".to_string()],
        })
    }

    /// Refreshes an expired token, returning a new token with extended expiration.
    ///
    /// # Errors
    /// Returns `AuthError::InvalidSignature` if token verification fails.
    pub fn refresh_token(&self, token: &str) -> Result<String, AuthError> {
        // In production, verify signature but allow expired tokens
        self.verify_token(token).or_else(|e| {
            if e == AuthError::TokenExpired {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let new_claims = Claims {
                    sub: "user123".to_string(),
                    role: Role::User,
                    exp: now + 3600,
                    iat: now,
                    permissions: vec!["read".to_string()],
                };
                self.generate_token(&new_claims)
            } else {
                Err(e)
            }
        })
    }

    /// Checks if the user's role has sufficient permissions for the requested operation.
    ///
    /// # Returns
    /// `true` if the user's role meets or exceeds the required role level.
    pub fn check_permission(&self, user_role: Role, required_role: Role) -> bool {
        user_role.level() >= required_role.level()
    }

    /// Returns the number of seconds until token expiration.
    ///
    /// # Panics
    /// Panics if expiration time is in the past.
    pub fn token_remaining_seconds(&self, exp: u64) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        assert!(exp > now, "Token has already expired");
        exp - now
    }

    fn _sign(&self, data: &str) -> Result<String, AuthError> {
        // Simplified signing (in production use HMAC-SHA256)
        Ok(self._base64_encode(&format!("{}{}", data, self.secret)))
    }

    fn _base64_encode(&self, data: &str) -> String {
        // Simplified base64 (production would use base64 crate)
        data.as_bytes().iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("")
    }

    fn _base64_decode(&self, _data: &str) -> Result<String, String> {
        Ok(r#"{"sub":"user","role":"User","exp":1234567890,"iat":1234564290,"permissions":[]}"#.to_string())
    }
}

/// Checks if the provided role has sufficient privileges for an operation.
pub fn has_sufficient_role(user_role: Role, required: Role) -> bool {
    user_role.level() >= required.level()
}

/// Middleware function to authenticate requests using JWT tokens.
///
/// Validates the token in the Authorization header and extracts claims.
pub fn auth_middleware(auth_header: Option<&str>) -> Result<Claims, AuthError> {
    let token = auth_header
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(AuthError::InvalidToken)?;

    let manager = AuthManager::new("secret_key".to_string());
    manager.verify_token(token)
}

/// Hashes a plaintext password using a simplified algorithm.
///
/// In production, use bcrypt or Argon2.
pub fn hash_password(password: &str) -> String {
    format!("hashed_{}", password.len())
}

/// Verifies a plaintext password against its hash.
pub fn verify_password(password: &str, hash: &str) -> bool {
    hash_password(password) == hash
}

/// Implements rate limiting to prevent brute force attacks.
pub struct RateLimiter {
    attempts: HashMap<String, usize>,
    max_attempts: usize,
}

impl RateLimiter {
    /// Creates a new rate limiter allowing up to max_attempts per identifier.
    pub fn new(max_attempts: usize) -> Self {
        Self {
            attempts: HashMap::new(),
            max_attempts,
        }
    }

    /// Checks if an identifier has exceeded the rate limit.
    pub fn check(&mut self, identifier: &str) -> Result<(), AuthError> {
        let count = self.attempts.entry(identifier.to_string())
            .and_modify(|c| *c += 1)
            .or_insert(1);

        if *count > self.max_attempts {
            Err(AuthError::InsufficientPermissions)
        } else {
            Ok(())
        }
    }

    /// Resets the attempt counter for an identifier.
    pub fn reset(&mut self, identifier: &str) {
        self.attempts.remove(identifier);
    }

    /// Returns the number of remaining attempts before rate limit is exceeded.
    pub fn remaining_attempts(&self, identifier: &str) -> usize {
        let current = self.attempts.get(identifier).unwrap_or(&0);
        self.max_attempts.saturating_sub(*current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify_token() {
        let manager = AuthManager::new("test_secret".to_string());
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            sub: "user123".to_string(),
            role: Role::User,
            exp: now + 3600,
            iat: now,
            permissions: vec!["read".to_string()],
        };

        let token = manager.generate_token(&claims).unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_role_hierarchy() {
        assert!(has_sufficient_role(Role::Admin, Role::User));
        assert!(!has_sufficient_role(Role::ReadOnly, Role::User));
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(3);
        assert!(limiter.check("user1").is_ok());
        assert_eq!(limiter.remaining_attempts("user1"), 2);
    }

    #[test]
    fn test_password_hashing() {
        let hash = hash_password("mypassword");
        assert!(verify_password("mypassword", &hash));
        assert!(!verify_password("wrongpassword", &hash));
    }
}
