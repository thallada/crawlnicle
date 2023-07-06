# crawlnicle

My personalized news and blog aggregator. Taking back control over the
algorithm. Pining for the days of Google Reader. An excuse to write more Rust.

## Development Instructions

### Prerequisites

Install these requirements to get started developing crawlnicle.

* [rust](https://www.rust-lang.org/)
* [postgres](https://www.postgresql.org/)
* [sqlx-cli](https://crates.io/crates/sqlx-cli)
  * Only postgres needed. Install with:

   ```bash
   cargo install sqlx-cli --no-default-features --features native-tls,postgres
   ```

* [just](https://github.com/casey/just#installation)
* (optional) [cargo-watch](https://github.com/watchexec/cargo-watch#install) for
auto-recompiling the server in development
* (optional) [mold](https://github.com/rui314/mold#installation) for faster
builds

### First-time setup

1. Create postgres user and database:

   ```bash
   createuser crawlnicle
   createdb crawlnicle
   sudo -u postgres -i psql
   postgres=# ALTER DATABASE crawlnicle OWNER TO crawlnicle;
   \password crawlnicle

   # Or, on Windows in PowerShell:

   & 'C:\Program Files\PostgreSQL\13\bin\createuser.exe' -U postgres crawlnicle
   & 'C:\Program Files\PostgreSQL\13\bin\createdb.exe' -U postgres crawlnicle
   & 'C:\Program Files\PostgreSQL\13\bin\psql.exe' -U postgres
   postgres=# ALTER DATABASE crawlnicle OWNER TO crawlnicle;
   \password crawlnicle
   ```

1. Save password somewhere safe and then and add a `.env` file to the project
   directory with the contents:

   ```env
   RUST_LOG=crawlnicle=debug,cli=debug,lib=debug,tower_http=debug,sqlx=debug
   HOST=127.0.0.1
   PORT=3000
   DATABASE_URL=postgresql://crawlnicle:<password>@localhost/crawlnicle
   DATABASE_MAX_CONNECTIONS=5
   TITLE=crawlnicle
   MAX_MEM_LOG_SIZE=1000000
   CONTENT_DIR=./content
   ```

1. Run `just migrate` (or `sqlx migrate run`) which will run all the database
   migrations.

### Running in Development

Run `just watch` to build and run the server while watching the source-files for
changes and triggering a recompilation when modifications are made.

The server also triggers the browser to reload the page when the server binary
is updated and the server is restarted.

It also separately watches the files in `frontend/` which will trigger a
transpilation with `bun` and then rebuild the server binary so that it includes
the new JS bundle names.

Alternatively, you can just run `cargo run` after building the frontend
JavaScript with `just build-dev-frontend`.

### Building for Production

You can also build the binary in release mode for running in production with the
`just build` command. This will first build the minified frontend JavaScript
(`just build-frontend`) and then build the rust binary with `cargo build
--release`.

## Using the CLI

This project also comes with a CLI binary which allows you to manipulate the
database directly without needing to go through the REST API server. Run
`cli --help` to see all of the available commands.

## Running jobs

To periodically fetch new items from all of the feeds execute the `cli crawl`
command in a cronjob.
