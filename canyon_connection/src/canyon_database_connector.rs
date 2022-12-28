use async_std::net::TcpStream;

use serde::Deserialize;
use tiberius::{AuthMethod, Config};
use tokio_postgres::{Client, NoTls};

use crate::datasources::DatasourceProperties;

/// Represents the current supported databases by Canyon
#[derive(Deserialize, Debug, Eq, PartialEq, Clone, Copy, Default)]
pub enum DatabaseType {
    #[default]
    #[serde(alias = "postgres", alias = "postgresql")]
    PostgreSql,
    #[serde(alias = "sqlserver", alias = "mssql")]
    SqlServer,
}

/// A connection with a `PostgreSQL` database
pub struct PostgreSqlConnection {
    pub client: Client,
    // pub connection: Connection<Socket, NoTlsStream>, // TODO Hold it, or not to hold it... that's the question!
}

/// A connection with a `SqlServer` database
pub struct SqlServerConnection {
    pub client: &'static mut tiberius::Client<TcpStream>,
}

/// The Canyon database connection handler. When the client's program
/// starts, Canyon gets the information about the desired datasources,
/// process them and generates a pool of 1 to 1 database connection for
/// every datasource defined.
pub struct DatabaseConnection {
    pub postgres_connection: Option<PostgreSqlConnection>,
    pub sqlserver_connection: Option<SqlServerConnection>,
    pub database_type: DatabaseType,
}

unsafe impl Send for DatabaseConnection {}
unsafe impl Sync for DatabaseConnection {}

impl DatabaseConnection {
    pub async fn new(
        datasource: &DatasourceProperties<'_>,
    ) -> Result<DatabaseConnection, Box<(dyn std::error::Error + Send + Sync + 'static)>> {
        match datasource.db_type {
            DatabaseType::PostgreSql => {
                let (new_client, new_connection) = tokio_postgres::connect(
                    &format!(
                        "postgres://{user}:{pswd}@{host}:{port}/{db}",
                        user = datasource.username,
                        pswd = datasource.password,
                        host = datasource.host,
                        port = datasource.port.unwrap_or_default(),
                        db = datasource.db_name
                    )[..],
                    NoTls,
                )
                .await?;

                tokio::spawn(async move {
                    if let Err(e) = new_connection.await {
                        eprintln!("An error occurred while trying to connect to the PostgreSQL database: {e}");
                    }
                });

                Ok(Self {
                    postgres_connection: Some(PostgreSqlConnection {
                        client: new_client,
                        // connection: new_connection,
                    }),
                    sqlserver_connection: None,
                    database_type: DatabaseType::PostgreSql,
                })
            }
            DatabaseType::SqlServer => {
                let mut config = Config::new();

                config.host(datasource.host);
                config.port(datasource.port.unwrap_or_default());
                config.database(datasource.db_name);

                // Using SQL Server authentication.
                config.authentication(AuthMethod::sql_server(
                    datasource.username,
                    datasource.password,
                ));

                // on production, it is not a good idea to do this. We should upgrade
                // Canyon in future versions to allow the user take care about this
                // configuration
                config.trust_cert();

                // Taking the address from the configuration, using async-std's
                // TcpStream to connect to the server.
                let tcp = TcpStream::connect(config.get_addr())
                    .await
                    .expect("Error instantiating the SqlServer TCP Stream");

                // We'll disable the Nagle algorithm. Buffering is handled
                // internally with a `Sink`.
                tcp.set_nodelay(true)
                    .expect("Error in the SqlServer `nodelay` config");

                // Handling TLS, login and other details related to the SQL Server.
                let client = tiberius::Client::connect(config, tcp).await;

                Ok(Self {
                    postgres_connection: None,
                    sqlserver_connection: Some(SqlServerConnection {
                        client: Box::leak(Box::new(
                            client.expect("A failure happened connecting to the database"),
                        )),
                    }),
                    database_type: DatabaseType::SqlServer,
                })
            }
        }
    }
}

#[cfg(test)]
mod database_connection_handler {
    use super::*;
    use crate::CanyonSqlConfig;

    const CONFIG_FILE_MOCK_ALT: &str = r#"
        [canyon_sql]
        datasources = [
            {name = 'PostgresDS', properties.db_type = 'postgresql', properties.username = 'username', properties.password = 'random_pass', properties.host = 'localhost', properties.db_name = 'triforce', properties.migrations='enabled'},
            {name = 'SqlServerDS', properties.db_type = 'sqlserver', properties.username = 'username2', properties.password = 'random_pass2', properties.host = '192.168.0.250.1', properties.port = 3340, properties.db_name = 'triforce2', properties.migrations='disabled'}
        ]
    "#;

    /// Tests the behaviour of the `DatabaseType::from_datasource(...)`
    #[test]
    fn check_from_datasource() {
        let config: CanyonSqlConfig = toml::from_str(CONFIG_FILE_MOCK_ALT)
            .expect("A failure happened retrieving the [canyon_sql] section");

        let psql_ds = &config.canyon_sql.datasources[0].properties;
        let sqls_ds = &config.canyon_sql.datasources[1].properties;

        assert_eq!(psql_ds.db_type, DatabaseType::PostgreSql);
        assert_eq!(sqls_ds.db_type, DatabaseType::SqlServer);
    }
}
