pub use sea_orm_migration::prelude::*;

mod m20220813_000001_create_entries_table;
mod m20220816_000001_create_users_table;
mod m20220819_000001_create_sessions_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220813_000001_create_entries_table::Migration),
            Box::new(m20220816_000001_create_users_table::Migration),
            Box::new(m20220819_000001_create_sessions_table::Migration),
        ]
    }
}
