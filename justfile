#!/usr/bin/env just --justfile

build: build-frontend
  cargo build --release

install-frontend:
  bun install --cwd frontend

clean-frontend:
  rm -rf ./static/js/* ./static/css/*

build-frontend: clean-frontend
  bun build frontend/js/index.ts \
    --outdir ./static \
    --root ./frontend \
    --entry-naming [dir]/[name]-[hash].[ext] \
    --asset-naming [dir]/[name]-[hash].[ext] \
    --minify

build-dev-frontend: clean-frontend
  bun build frontend/js/index.ts \
    --outdir ./static \
    --root ./frontend \
    --entry-naming [dir]/[name]-[hash].[ext] \
    --asset-naming [dir]/[name]-[hash].[ext]
  touch .frontend-built # triggers watch-backend since static/* is ignored

watch-frontend: install-frontend
  cargo watch -w frontend \
    -s 'just build-dev-frontend'

watch-backend:
  mold -run cargo watch \
    --ignore 'logs/*' \
    --ignore 'static/*' \
    --ignore 'frontend/*' \
    --no-vcs-ignores \
    -x run

# runs watch-frontend and watch-backend simultaneously
watch:
 ./watch.sh

migrate:
  sqlx migrate run
