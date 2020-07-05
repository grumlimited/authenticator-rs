pub mod refinery {
    use refinery::embed_migrations;

    embed_migrations!("build-aux/migrations");
}

pub mod runner {
    use refinery::{Error, Report};
    use rusqlite::Connection;

    pub fn run(connection: &mut Connection) -> Result<Report, Error> {
        crate::helpers::refinery::migrations::runner().run(connection)
    }
}
