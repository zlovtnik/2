use async_trait::async_trait;
use sqlx::{PgPool, FromRow, Error};
use std::marker::PhantomData;
use tracing::{info, warn, error, debug};

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

impl<T> PgCrud<T>
where
    T: Send + Sync + Unpin + for<'r> FromRow<'r, sqlx::postgres::PgRow> + 'static,
{
    // Remove the create_with helper. No generic insert helper is provided.
}

#[async_trait]
impl<T, Id> Crud<T, Id> for PgCrud<T>
where
    T: Send + Sync + Unpin + for<'r> FromRow<'r, sqlx::postgres::PgRow> + 'static,
    Id: Send + Sync + sqlx::Type<sqlx::Postgres> + sqlx::Encode<'static, sqlx::Postgres> + 'static,
{
    async fn create(&self, _entity: &T) -> Result<T, Error> {
        warn!(table = %self.table, "Create method called but not implemented");
        unimplemented!("Provide entity-specific insert logic using closures or higher-order functions")
    }
    async fn read(&self, id: Id) -> Result<Option<T>, Error> {
        debug!(table = %self.table, "Starting database read operation");
        let query = format!("SELECT * FROM {} WHERE id = $1", self.table);
        debug!(table = %self.table, query = %query, "Constructed read query");
        
        let query: &'static str = Box::leak(query.into_boxed_str());
        let row = sqlx::query_as::<_, T>(query)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!(table = %self.table, error = %e, "Database read operation failed");
                e
            })?;
        
        match &row {
            Some(_) => {
                debug!(table = %self.table, "Database read operation successful - record found");
            },
            None => {
                debug!(table = %self.table, "Database read operation successful - no record found");
            }
        }
        
        Ok(row)
    }
    async fn delete(&self, id: Id) -> Result<u64, Error> {
        debug!(table = %self.table, "Starting database delete operation");
        let query = format!("DELETE FROM {} WHERE id = $1", self.table);
        debug!(table = %self.table, query = %query, "Constructed delete query");
        
        let query: &'static str = Box::leak(query.into_boxed_str());
        let result = sqlx::query(query)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!(table = %self.table, error = %e, "Database delete operation failed");
                e
            })?;
        
        let rows_affected = result.rows_affected();
        if rows_affected > 0 {
            info!(table = %self.table, rows_affected = rows_affected, "Database delete operation successful");
        } else {
            debug!(table = %self.table, "Database delete operation completed - no rows affected");
        }
        
        Ok(rows_affected)
    }
}

#[async_trait]
impl<T, Id> UpdatableCrud<T, Id> for PgCrud<T>
where
    T: Send + Sync + Unpin + for<'r> FromRow<'r, sqlx::postgres::PgRow> + 'static,
    Id: Send + Sync + sqlx::Type<sqlx::Postgres> + sqlx::Encode<'static, sqlx::Postgres> + 'static,
{
    async fn update(&self, _id: Id, _update_fn: impl FnOnce(T) -> T + Send) -> Result<Option<T>, Error> {
        warn!(table = %self.table, "Update method called but not implemented");
        unimplemented!("Provide entity-specific update logic using closures or higher-order functions")
    }
} 