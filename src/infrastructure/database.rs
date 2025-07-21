use async_trait::async_trait;
use sqlx::{PgPool, FromRow, Error};
use std::marker::PhantomData;

#[async_trait]
pub trait Crud<T, Id>
where
    T: Send + Sync + Unpin + for<'r> FromRow<'r, sqlx::postgres::PgRow> + 'static,
    Id: Send + Sync + 'static,
{
    async fn create(&self, entity: &T) -> Result<T, Error>;
    async fn read(&self, id: Id) -> Result<Option<T>, Error>;
    async fn delete(&self, id: Id) -> Result<u64, Error>;
}

#[async_trait]
pub trait UpdatableCrud<T, Id>: Crud<T, Id>
where
    T: Send + Sync + Unpin + for<'r> FromRow<'r, sqlx::postgres::PgRow> + 'static,
    Id: Send + Sync + 'static,
{
    async fn update(&self, id: Id, update_fn: impl FnOnce(T) -> T + Send) -> Result<Option<T>, Error>;
}

pub struct PgCrud<T> {
    pub pool: PgPool,
    pub table: String,
    _marker: PhantomData<T>,
}

impl<T> PgCrud<T> {
    pub fn new(pool: PgPool, table: &'static str) -> Self {
        Self { pool, table: table.to_string(), _marker: PhantomData }
    }
}

#[async_trait]
impl<T, Id> Crud<T, Id> for PgCrud<T>
where
    T: Send + Sync + Unpin + for<'r> FromRow<'r, sqlx::postgres::PgRow> + 'static,
    Id: Send + Sync + sqlx::Type<sqlx::Postgres> + sqlx::Encode<'static, sqlx::Postgres> + 'static,
{
    async fn create(&self, _entity: &T) -> Result<T, Error> {
        unimplemented!("Provide entity-specific insert logic using closures or higher-order functions")
    }
    async fn read(&self, id: Id) -> Result<Option<T>, Error> {
        let query = format!("SELECT * FROM {} WHERE id = $1", self.table);
        let query: &'static str = Box::leak(query.into_boxed_str());
        let row = sqlx::query_as::<_, T>(query)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }
    async fn delete(&self, id: Id) -> Result<u64, Error> {
        let query = format!("DELETE FROM {} WHERE id = $1", self.table);
        let query: &'static str = Box::leak(query.into_boxed_str());
        let result = sqlx::query(query)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}

#[async_trait]
impl<T, Id> UpdatableCrud<T, Id> for PgCrud<T>
where
    T: Send + Sync + Unpin + for<'r> FromRow<'r, sqlx::postgres::PgRow> + 'static,
    Id: Send + Sync + sqlx::Type<sqlx::Postgres> + sqlx::Encode<'static, sqlx::Postgres> + 'static,
{
    async fn update(&self, _id: Id, _update_fn: impl FnOnce(T) -> T + Send) -> Result<Option<T>, Error> {
        unimplemented!("Provide entity-specific update logic using closures or higher-order functions")
    }
} 