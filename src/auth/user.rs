use axum::extract::FromRequestParts;
use axum_extra::extract::CookieJar;
use core::panic;
use jwt_simple::{
    algorithms::{HS256Key, MACLike},
    claims::Claims,
    reexports::coarsetime::Duration,
};
use password_auth::VerifyError;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;

use crate::{app::AppState, error::AppError, repository::Repository};
const SECRET_KEY: &[u8] = b"im-so-secret";
pub struct UnauthenticatedUser {
    username: String,
    password: String,
}
impl UnauthenticatedUser {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }
    pub async fn authenticate(&self, repository: &Repository) -> Result<User, AppError> {
        let user_record = match repository.get_user_by_name(&self.username).await? {
            Some(user_record) => user_record,
            None => return Err(AppError::UserDoesNotExist),
        };
        match password_auth::verify_password(&self.password, &user_record.password_hash) {
            Ok(()) => Ok(User::new(user_record.id, user_record.username)),
            Err(VerifyError::PasswordInvalid) => Err(AppError::InvalidCredentials),
            Err(VerifyError::Parse(err)) => panic!("Hashing algorithm failed: {err}"),
        }
    }
    pub async fn register(self, repository: &Repository) -> Result<User, AppError> {
        let password_hash = password_auth::generate_hash(self.password);
        let user_record = match repository.add_user(&self.username, &password_hash).await {
            Ok(user_record) => user_record,
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                return Err(AppError::UserNameToken);
            }
            Err(err) => return Err(AppError::Database(err)),
        };
        Ok(User::new(user_record.id, user_record.username))
    }
}
pub struct User {
    pub id: i64,
    pub username: String,
}

impl User {
    fn new(id: i64, username: String) -> Self {
        Self { id, username }
    }
    pub const fn username(&self) -> &String {
        &self.username
    }
    pub const fn id(&self) -> i64 {
        self.id
    }
    pub fn auth_token(self) -> Result<String, AppError> {
        let key = HS256Key::from_bytes(SECRET_KEY);
        let claims = Claims::with_custom_claims(UserClaims::from(self), Duration::from_mins(10));
        let token = key.authenticate(claims)?;
        Ok(token.to_string())
    }
    pub fn from_auth_token(token: &str) -> Result<Self, AppError> {
        let key = HS256Key::from_bytes(SECRET_KEY);
        let claims: UserClaims = key.verify_token(token, None)?.custom;
        Ok(Self::new(claims.id, claims.username))
    }
}

impl FromRequestParts<AppState> for User {
    type Rejection = AppError;
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = match jar.get("token") {
            Some(token) => token.value(),
            None => return Err(AppError::MissingAuthorization),
        };
        User::from_auth_token(token)
    }
}
impl FromRequestParts<AppState> for Option<User> {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(User::from_request_parts(parts, state).await.ok())
    }
}

#[derive(Serialize, Deserialize)]
struct UserClaims {
    id: i64,
    username: String,
}

impl From<User> for UserClaims {
    fn from(User { id, username }: User) -> Self {
        Self { id, username }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn register_creates_a_new_user(pool: PgPool) -> sqlx::Result<()> {
        let repository = Repository::new(pool);
        let unauthenticated =
            UnauthenticatedUser::new("maria".to_string(), "senha-forte".to_string());

        let user = unauthenticated
            .register(&repository)
            .await
            .expect("deveria registrar com sucesso");

        assert_eq!(user.username(), "maria");
        Ok(())
    }

    #[sqlx::test]
    async fn register_fails_for_duplicate_username(pool: PgPool) -> sqlx::Result<()> {
        let repository = Repository::new(pool);
        UnauthenticatedUser::new("duplicado".to_string(), "senha-1".to_string())
            .register(&repository)
            .await
            .expect("primeiro registro deveria funcionar");

        let result = UnauthenticatedUser::new("duplicado".to_string(), "senha-2".to_string())
            .register(&repository)
            .await;

        assert!(matches!(result, Err(AppError::UserNameToken)));
        Ok(())
    }

    #[sqlx::test]
    async fn authenticate_succeeds_with_correct_password(pool: PgPool) -> sqlx::Result<()> {
        let repository = Repository::new(pool);
        UnauthenticatedUser::new("joao".to_string(), "12345678".to_string())
            .register(&repository)
            .await
            .expect("deveria registrar com sucesso");

        let user = UnauthenticatedUser::new("joao".to_string(), "12345678".to_string())
            .authenticate(&repository)
            .await
            .expect("deveria autenticar com sucesso");

        assert_eq!(user.username(), "joao");
        Ok(())
    }

    #[sqlx::test]
    async fn authenticate_fails_with_wrong_password(pool: PgPool) -> sqlx::Result<()> {
        let repository = Repository::new(pool);
        UnauthenticatedUser::new("pedro".to_string(), "senha-correta".to_string())
            .register(&repository)
            .await
            .expect("deveria registrar com sucesso");

        let result = UnauthenticatedUser::new("pedro".to_string(), "senha-errada".to_string())
            .authenticate(&repository)
            .await;

        assert!(matches!(result, Err(AppError::InvalidCredentials)));
        Ok(())
    }

    #[sqlx::test]
    async fn authenticate_fails_when_user_does_not_exist(pool: PgPool) -> sqlx::Result<()> {
        let repository = Repository::new(pool);

        let result = UnauthenticatedUser::new("fantasma".to_string(), "qualquer-coisa".to_string())
            .authenticate(&repository)
            .await;

        assert!(matches!(result, Err(AppError::UserDoesNotExist)));
        Ok(())
    }

    #[test]
    fn jwt_round_trip_preserves_user_data() {
        let user = User::new(42, "carla".to_string());
        let token = user.auth_token().expect("deveria gerar o token");

        let restored = User::from_auth_token(&token).expect("deveria decodificar o token");
        assert_eq!(restored.id(), 42);
        assert_eq!(restored.username(), "carla");
    }
}
