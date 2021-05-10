use tokio_postgres::{Client, NoTls, Error as PgError};
use async_trait::async_trait;

pub struct Contact {
    pub id: i32,
    pub firstname: String,
    pub lastname: String,
    pub phone: String,
    pub email: String
}

#[async_trait]
pub trait Repository {
    async fn new(dsl: &str) -> Self;
    async fn get(&self, id: i32) -> Result<Contact, Error>;
    async fn save(&self, contact: &Contact) -> Result<u64, Error>;
}

pub struct PgsqlRepository {
    client: Client
}

#[derive(Debug)]
pub enum Error {
    Db(PgError),
    Intern(String),
}

impl From<PgError> for Error {
    fn from(err: PgError) -> Error {
        Error::Db(err)
    }
}

impl From<String> for Error {
    fn from(err: String) -> Error {
        Error::Intern(err)
    }
}

#[async_trait]
impl Repository for PgsqlRepository {
    async fn new(dsn: &str) -> Self {
        let (client, connection) = tokio_postgres::connect(dsn, NoTls).await.unwrap();

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        Self { client }
    }

    async fn get(&self, id: i32) -> Result<Contact, Error> {
        let rows = self.client.query("SELECT id, firstname, lastname, phone, email FROM contact WHERE id=$1", &[&id]).await?;
        if rows.len() == 0 {
            Err(Error::Intern(format!("no record with id {}", id)))
        } else {
            Ok(Contact { id: rows[0].get(0), firstname: rows[0].get(1),
                lastname: rows[0].get(2), phone: rows[0].get(3), email: rows[0].get(4)
            })
        }
    }

    async fn save(&self, contact: &Contact) -> Result<u64, Error> {
        Ok(self.client.execute("INSERT INTO contact (id, firstname, lastname, phone, email) VALUES ($1, $2, $3, $4, $5)",
                            &[&contact.id, &contact.firstname, &contact.lastname, &contact.phone, &contact.email]).await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::repository::{PgsqlRepository, Repository, Contact};
    use test_context::{test_context, AsyncTestContext};
    use tokio_postgres::{NoTls};
    use async_trait::async_trait;

    struct PgContext { repository: PgsqlRepository }

    #[async_trait]
    impl AsyncTestContext for PgContext {
        async fn setup() -> PgContext {
            let (client, connection) = tokio_postgres::connect("host=postgresql user=test password=test dbname=test", NoTls).await.unwrap();

            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });
            PgContext {  repository: PgsqlRepository{ client } }
        }

        async fn teardown(self) {
            self.repository.client.execute("DELETE FROM contact", &[]).await.unwrap();
        }
    }

    #[test_context(PgContext)]
    #[tokio::test]
    async fn get_contact_no_contact(ctx: &PgContext) {
        assert!(ctx.repository.get(12).await.is_err(), "no results should be found")
    }

    #[test_context(PgContext)]
    #[tokio::test]
    async fn save_get_contact(ctx: &PgContext) {
        let contact = Contact {
            id: 13,
            firstname: "first".to_string(),
            lastname: "second".to_string(),
            phone: "0123456789".to_string(),
            email: "e@mail.com".to_string()
        };
        assert!(ctx.repository.save(&contact).await.is_ok(), "save should succeed");
        assert!(ctx.repository.get(13).await.is_ok(), "contact should be found")
    }


}