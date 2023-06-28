#!/usr/bin/env just --justfile

build: build-frontend
  cargo build --release

install-frontend:
  bun install --cwd frontend

clean-frontend:
  rm -rf ./static/js/*

build-frontend: install-frontend clean-frontend
  bun build frontend/index.ts \
    --outdir ./static/js \
    --entry-naming [name]-[hash].[ext]

watch-frontend: install-frontend
  cargo watch -w frontend \
    -s 'rm -rf ./static/js/*' \
    -s 'bun build frontend/index.ts \
    --outdir ./static/js \
    --entry-naming [name]-[hash].[ext]' \
    -s 'touch .frontend-built'

watch-backend:
  mold -run cargo watch \
    --ignore 'logs/*' \
    --ignore 'static/*' \
    --ignore 'frontend/*' \
    -x run

watch:
 ./watch.sh

migrate:
  sqlx migrate run
