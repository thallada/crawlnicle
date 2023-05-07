CREATE TABLE IF NOT EXISTS "items" (
    "id" SERIAL PRIMARY KEY NOT NULL,
    "title" VARCHAR(255) NOT NULL,
    "url" VARCHAR(255) NOT NULL,
    "description" VARCHAR(255),
    "created_at" timestamp(3) NOT NULL,
    "updated_at" timestamp(3) NOT NULL,
    "deleted_at" timestamp(3)
);
