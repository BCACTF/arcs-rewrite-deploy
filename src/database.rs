use tokio_postgres::{ NoTls, Client };

use std::io::{Result as IOResult, Error as IOError, ErrorKind};

#[allow(unused_imports)]
use crate::logging::{ trace, debug, info, warn, error };


pub async fn database_init() -> IOResult<Client> {
    let postgres_result = tokio_postgres::connect("host=localhost user=ricecrispieismyname_gmail_com password=rsgRNnzdLtxY7gELJEn.NC@B", NoTls).await;
    let postgres_ok_result = match postgres_result {
        Ok(postgres_ok_result) => postgres_ok_result,
        Err(err) => {
            error!("Error connecting to postgres server!");
            info!("Postgres error: {}", err);
            return Err(IOError::new(ErrorKind::Other, err))
        }
    };
    let (client, connection) = postgres_ok_result;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("Connection error: {}", e);
        }
    });

    Ok(client)
}
