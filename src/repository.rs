use std::convert::Infallible;

use axum::extract::FromRequestParts;
use sqlx::PgPool;

use crate::{
    app::AppState,
    model::{Asset, OwnedAsset, UserRecord},
};

pub struct Repository {
    db: PgPool,
}

impl Repository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn list_assets(&self) -> sqlx::Result<Vec<Asset>> {
        sqlx::query_as!(Asset, "SELECT id,name,unit_value,coingecko_id FROM assets;")
            .fetch_all(&self.db)
            .await
    }
    pub async fn create_asset(
        &self,
        name: String,
        unit_value: f64,
        coingecko_id: Option<String>,
    ) -> sqlx::Result<Asset> {
        sqlx::query_as!(
            Asset,
            "INSERT INTO assets (name, unit_value, coingecko_id)
            VALUES ($1,$2, $3)
            RETURNING  id, name, unit_value, coingecko_id;",
            name,
            unit_value,
            coingecko_id,
        )
        .fetch_one(&self.db)
        .await
    }
    pub async fn update_asset(
        &self,
        id: i64,
        name: Option<String>,
        unit_value: Option<f64>,
        coingecko_id: Option<String>,
    ) -> sqlx::Result<Option<Asset>> {
        sqlx::query_as!(
            Asset,
            "UPDATE assets
                SET name=COALESCE($2, name), 
                    unit_value=COALESCE($3, unit_value),
                    coingecko_id=COALESCE($4, coingecko_id)
            WHERE id=$1
            RETURNING  id, name, unit_value, coingecko_id;",
            id,
            name,
            unit_value,
            coingecko_id
        )
        .fetch_optional(&self.db)
        .await
    }
    pub async fn add_user(&self, username: &str, password_hash: &str) -> sqlx::Result<UserRecord> {
        sqlx::query_as!(
            UserRecord,
            "INSERT INTO users (username, password_hash)
            VALUES ($1,$2)
            RETURNING  id, username, password_hash;",
            username,      //inputs
            password_hash, //inputs
        )
        .fetch_one(&self.db)
        .await
    }
    pub async fn get_user_by_name(&self, username: &str) -> sqlx::Result<Option<UserRecord>> {
        sqlx::query_as!(
            UserRecord,
            "SELECT id, username, password_hash
            FROM users
            WHERE username=$1;",
            username,
        )
        .fetch_optional(&self.db)
        .await
    }
    pub async fn list_owned_assets(&self, user_id: i64) -> sqlx::Result<Vec<OwnedAsset>> {
        sqlx::query_as!(
            OwnedAsset,
            r#"
            SELECT
            a.id,
            a.name, 
            a.unit_value,
            SUM((a.unit_value - o.bought_for) * o.quantity_owned) AS "value_delta!",
            SUM(o.quantity_owned) AS "quantity_owned!",
            JSON_AGG(
                JSON_BUILD_OBJECT(
                    'bought_at', o.timestamp,
                    'bought_for', o.bought_for,
                    'quantity_bought', o.quantity_owned,
                    'value_delta', (a.unit_value - o.bought_for) * o.quantity_owned
                )
            ) AS "purchase_history!: _"
            FROM assets AS a
            JOIN owned_assets AS o
              ON o.assets_id = a.id
            WHERE o.user_id = $1
            GROUP BY a.id;
            "#,
            user_id
        )
        .fetch_all(&self.db)
        .await
    }

    pub async fn insert_owned_asset(
        &self,
        user_id: i64,
        asset_id: i64,
        quantity: f64,
        unit_value: f64,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO owned_assets (user_id,assets_id,quantity_owned, bought_for) VALUES ($1, $2,$3, $4);",
            user_id,
            asset_id,
            quantity,
            unit_value,
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }
    pub async fn list_assets_with_coingecko_id(&self) -> sqlx::Result<Vec<Asset>> {
        sqlx::query_as!(
            Asset,
            "SELECT id, name, unit_value, coingecko_id
            FROM assets
            WHERE coingecko_id IS NOT NULL;"
        )
        .fetch_all(&self.db)
        .await
    }
    pub async fn update_asset_price(&self, id: i64, unit_value: f64) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE assets SET unit_value = $2 WHERE id = $1;",
            id,
            unit_value,
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }
}

impl FromRequestParts<AppState> for Repository {
    type Rejection = Infallible;

    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self {
            db: state.db.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[sqlx::test]
    async fn create_and_list_asset(pool: PgPool) -> sqlx::Result<()> {
        let repository = Repository::new(pool);

        let created = repository
            .create_asset("Bitcoin".to_string(), 350_000.0, None)
            .await?;
        assert_eq!(created.name, "Bitcoin");
        assert_eq!(created.unit_value, 350_000.0);

        let assets = repository.list_assets().await?;
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].id, created.id);

        Ok(())
    }

    #[sqlx::test]
    async fn update_existing_asset(pool: PgPool) -> sqlx::Result<()> {
        let repository = Repository::new(pool);
        let created = repository
            .create_asset("Ethereum".to_string(), 12_000.0, None)
            .await?;

        let updated = repository
            .update_asset(created.id, None, Some(13_500.0), None)
            .await?
            .expect("o ativo deveria existir");

        assert_eq!(updated.name, "Ethereum");
        assert_eq!(updated.unit_value, 13_500.0);

        Ok(())
    }

    #[sqlx::test]
    async fn update_missing_asset_returns_none(pool: PgPool) -> sqlx::Result<()> {
        let repository = Repository::new(pool);

        let result = repository
            .update_asset(999, Some("Fantasma".to_string()), None, None)
            .await?;

        assert!(result.is_none());
        Ok(())
    }

    #[sqlx::test]
    async fn add_and_fetch_user(pool: PgPool) -> sqlx::Result<()> {
        let repository = Repository::new(pool);

        let created = repository.add_user("rafael", "hashed-password").await?;
        assert_eq!(created.username, "rafael");

        let fetched = repository
            .get_user_by_name("rafael")
            .await?
            .expect("o usuário deveria existir");
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.password_hash, "hashed-password");

        let missing = repository.get_user_by_name("desconhecido").await?;
        assert!(missing.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn duplicate_username_fails(pool: PgPool) -> sqlx::Result<()> {
        let repository = Repository::new(pool);

        repository.add_user("duplicado", "hash-1").await?;
        let result = repository.add_user("duplicado", "hash-2").await;

        assert!(result.is_err());
        Ok(())
    }

    #[sqlx::test]
    async fn insert_and_list_owned_assets(pool: PgPool) -> sqlx::Result<()> {
        let repository = Repository::new(pool);

        let user = repository.add_user("investidor", "hash").await?;
        let asset = repository
            .create_asset("Ouro".to_string(), 350.0, None)
            .await?;

        repository
            .insert_owned_asset(user.id, asset.id, 10.0, 300.0)
            .await?;

        let owned = repository.list_owned_assets(user.id).await?;
        assert_eq!(owned.len(), 1);

        let owned_asset = &owned[0];
        assert_eq!(owned_asset.quantity_owned, 10.0);
        // (unit_value - bought_for) * quantity = (350 - 300) * 10 = 500
        assert_eq!(owned_asset.value_delta, 500.0);
        assert_eq!(owned_asset.purchase_history.0.len(), 1);
        assert_eq!(owned_asset.purchase_history.0[0].quantity_bought, 10.0);

        Ok(())
    }

    #[sqlx::test]
    async fn list_owned_assets_is_empty_for_new_user(pool: PgPool) -> sqlx::Result<()> {
        let repository = Repository::new(pool);
        let user = repository.add_user("sem-ativos", "hash").await?;

        let owned = repository.list_owned_assets(user.id).await?;
        assert!(owned.is_empty());

        Ok(())
    }
}
