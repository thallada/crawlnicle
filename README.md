# crawlnicle

My personalized news and blog aggregator. Taking back control over the
algorithm. Pining for the days of Google Reader. An excuse to write more Rust.

## Development Install

1. Install and run postgres.
2. Create postgres user and database:

```
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

3. Save password somewhere safe and then and add a `.env` file to the project
   directory with the contents:

```
RUST_LOG=crawlnicle=debug,cli=debug,lib=debug,tower_http=debug,sqlx=debug
HOST=127.0.0.1
PORT=3000
DATABASE_URL=postgresql://crawlnicle:<password>@localhost/crawlnicle
DATABASE_MAX_CONNECTIONS=5
TITLE=crawlnicle
MAX_MEM_LOG_SIZE=1000000
```

4. Install
   [`sqlx_cli`](https://github.com/launchbadge/sqlx/tree/master/sqlx-cli) with
   `cargo install sqlx-cli --no-default-features --features native-tls,postgres`
5. Run `sqlx migrate run` which will run all the database migrations.
6. Build the release binary by running `cargo build --release`.
7. Run `./target/build/crawlnicle` to start the server.

## Using the CLI

This project also comes with a CLI binary which allows you to manipulate the
database directly without needing to go through the REST API server. Run
`./target/build/cli --help` to see all of the available commands.

## Running jobs

To periodically fetch new items from all of the feeds execute the `cli crawl`
command in a cronjob.
