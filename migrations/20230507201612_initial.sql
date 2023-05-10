CREATE TYPE feed_type AS ENUM ('atom', 'rss');

CREATE TABLE IF NOT EXISTS "feeds" (
    "id" SERIAL PRIMARY KEY NOT NULL,
    "title" VARCHAR(255) NOT NULL,
    "url" VARCHAR(2048) NOT NULL,
    "type" feed_type NOT NULL,
    "description" TEXT,
    "created_at" timestamp(3) NOT NULL,
    "updated_at" timestamp(3) NOT NULL,
    "deleted_at" timestamp(3)
);
CREATE INDEX "feeds_deleted_at" ON "feeds" ("deleted_at");
CREATE UNIQUE INDEX "feeds_url" ON "feeds" ("url");

CREATE TABLE IF NOT EXISTS "items" (
    "id" SERIAL PRIMARY KEY NOT NULL,
    "title" VARCHAR(255) NOT NULL,
    "url" VARCHAR(2048) NOT NULL,
    "description" TEXT,
    "feed_id" INTEGER REFERENCES "feeds"(id) NOT NULL,
    "created_at" timestamp(3) NOT NULL,
    "updated_at" timestamp(3) NOT NULL,
    "deleted_at" timestamp(3)
);
CREATE INDEX "items_deleted_at" ON "items" ("deleted_at");
CREATE UNIQUE INDEX "items_url_and_feed_id" ON "items" ("url", "feed_id");
