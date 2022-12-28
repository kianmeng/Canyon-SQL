pub mod delete_operations;
pub mod foreign_key_operations;
pub mod insert_operations;
pub mod querybuilder_operations;
pub mod select_operations;
pub mod update_operations;

use crate::constants::SQL_SERVER_CREATE_TABLES;
use crate::constants::SQL_SERVER_DS;
use crate::constants::SQL_SERVER_FILL_TABLE_VALUES;
use crate::tests_models::league::League;

use canyon_sql::crud::CrudOperations;
use canyon_sql::db_clients::tiberius::{Client, Config};
use canyon_sql::runtime::tokio::net::TcpStream;
use canyon_sql::runtime::tokio_util::compat::TokioAsyncWriteCompatExt;

/// In order to initialize data on `SqlServer`. we must manually insert it
/// when the docker starts. SqlServer official docker from Microsoft does
/// not allow you to run `.sql` files against the database (not at least, without)
/// using a workaround. So, we are going to query the `SqlServer` to check if already
/// has some data (other processes, persistence or multi-threading envs), af if not,
/// we are going to retrieve the inserted data on the `postgreSQL` at start-up and
/// inserting into the `SqlServer` instance.
///
/// This will be marked as `#[ignore]`, so we can force to run first the marked as
/// ignored, check the data available, perform the necessary init operations and
/// then *cargo test <args...>* the real integration tests
#[canyon_sql::macros::canyon_tokio_test]
#[ignore]
fn initialize_sql_server_docker_instance() {
    canyon_sql::runtime::futures::executor::block_on(async {
        static CONN_STR: &str =
            "server=tcp:localhost,1434;User Id=SA;Password=SqlServer-10;TrustServerCertificate=true";

        let config = Config::from_ado_string(CONN_STR).unwrap();

        let tcp = TcpStream::connect(config.get_addr()).await.unwrap();
        let tcp2 = TcpStream::connect(config.get_addr()).await.unwrap();
        tcp.set_nodelay(true).ok();

        let mut client = Client::connect(config.clone(), tcp.compat_write())
            .await
            .unwrap();

        // Create the tables
        let query_result = client.query(SQL_SERVER_CREATE_TABLES, &[]).await;
        assert!(query_result.is_ok());

        let leagues_sql = League::find_all_datasource(SQL_SERVER_DS).await;
        println!("LSQL ERR: {leagues_sql:?}");
        assert!(leagues_sql.is_ok());

        match leagues_sql {
            Ok(ref leagues) => {
                let leagues_len = leagues.len();
                println!("Leagues already inserted on SQLSERVER: {:?}", &leagues_len);
                if leagues.len() < 10 {
                    let mut client2 = Client::connect(config, tcp2.compat_write())
                        .await
                        .expect("Can't connect to MSSQL");
                    let result = client2.query(SQL_SERVER_FILL_TABLE_VALUES, &[]).await;
                    assert!(result.is_ok());
                }
            }
            Err(e) => eprintln!("Error retrieving the leagues: {e}"),
        }
    });
}
