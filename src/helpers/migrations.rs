pub mod refinery {
    use refinery::embed_migrations;

    embed_migrations!("build-aux/migrations");
}

pub mod runner {
    use refinery::{Error, Report};
    use rusqlite::Connection;
    use std::ops::DerefMut;
    use std::sync::{Arc, Mutex};

    pub fn run(connection: Arc<Mutex<Connection>>) -> Result<Report, Error> {
        let mut conn = connection.lock().unwrap();
        let conn = conn.deref_mut();
        crate::helpers::refinery::migrations::runner().run(conn)
    }
}
