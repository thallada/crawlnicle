-- This extension gives us `uuid_generate_v1mc()` which generates UUIDs that cluster better than `gen_random_uuid()`
-- while still being difficult to predict and enumerate.
-- Also, while unlikely, `gen_random_uuid()` can in theory produce collisions which can trigger spurious errors on
-- insertion, whereas it's much less likely with `uuid_generate_v1mc()`.
create extension if not exists "uuid-ossp";

-- Set up trigger to auto-set `updated_at` columns when rows are modified
create or replace function set_updated_at()
    returns trigger as
$$
begin
    NEW.updated_at = now();
    return NEW;
end;
$$ language plpgsql;

create or replace function trigger_updated_at(tablename regclass)
        returns void as
$$
begin
    execute format('CREATE TRIGGER set_updated_at
        BEFORE UPDATE
        ON %s
        FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();', tablename);
end;
$$ language plpgsql;

-- This is a text collation that sorts text case-insensitively, useful for `UNIQUE` indexes
-- over things like usernames and emails, ithout needing to remember to do case-conversion.
create collation case_insensitive (provider = icu, locale = 'und-u-ks-level2', deterministic = false);

create type feed_type as enum ('atom', 'json', 'rss0', 'rss1', 'rss2', 'unknown');

create table if not exists "feed" (
    feed_id uuid primary key default uuid_generate_v1mc(),
    title text,
    url varchar(2048) not null,
    type feed_type not null default 'unknown',
    description text default null,
    crawl_interval_minutes int not null default 180,
    last_crawl_error text default null,
    etag_header text default null,
    last_modified_header text default null,
    last_crawled_at timestamptz default null,
    last_entry_published_at timestamptz default null,
    created_at timestamptz not null default now(),
    updated_at timestamptz,
    deleted_at timestamptz
);
create index on "feed" (deleted_at);
create unique index on "feed" (url);
select trigger_updated_at('"feed"');

create table if not exists "entry" (
    entry_id uuid primary key default uuid_generate_v1mc(),
    title text,
    url varchar(2048) not null,
    description text,
    feed_id uuid not null references "feed" (feed_id) on delete cascade,
    etag_header text default null,
    last_modified_header text default null,
    published_at timestamptz not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz,
    deleted_at timestamptz
);
create index on "entry" (published_at desc) where deleted_at is null;
create unique index on "entry" (url, feed_id);
select trigger_updated_at('"entry"');
