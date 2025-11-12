pub mod refinery {
    use refinery::embed_migrations;

    embed_migrations!("build-aux/migrations");
}

pub mod runner {
    use refinery::{Error, Report};
    use rusqlite::Connection;
    use std::ops::DerefMut;
    use std::sync::{Arc, Mutex};

    #[allow(clippy::result_large_err)]
    pub fn run(connection: Arc<Mutex<Connection>>) -> Result<Report, Error> {
        let mut connection = connection.lock().unwrap();
        let connection = connection.deref_mut();
        crate::helpers::refinery::migrations::runner().run(connection)
    }
}
