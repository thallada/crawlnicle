build: build-frontend
  cargo build --release

install-frontend:
  bun install --cwd frontend

clean-frontend:
  rm -rf ./static/js/* ./static/css/* ./static/img/*

build-frontend: clean-frontend
  bunx tailwindcss -i frontend/css/styles.css -o static/css/styles.css --minify
  bun build frontend/js/index.ts \
    --outdir ./static \
    --root ./frontend \
    --entry-naming [dir]/[name]-[hash].[ext] \
    --chunk-naming [dir]/[name]-[hash].[ext] \
    --asset-naming [dir]/[name]-[hash].[ext] \
    --minify
  mkdir -p static/img
  cp frontend/img/* static/img/
  touch ./static/js/manifest.txt # create empty manifest to be overwritten by build.rs
  touch ./static/css/manifest.txt # create empty manifest to be overwritten by build.rs
  touch .frontend-built # trigger build.rs to run

build-dev-frontend: clean-frontend
  bunx tailwindcss -i frontend/css/styles.css -o static/css/styles.css
  bun build frontend/js/index.ts \
    --outdir ./static \
    --root ./frontend \
    --entry-naming [dir]/[name]-[hash].[ext] \
    --chunk-naming [dir]/[name]-[hash].[ext] \
    --asset-naming [dir]/[name]-[hash].[ext]
  mkdir -p static/img
  cp frontend/img/* static/img/
  touch ./static/js/manifest.txt # create empty manifest needed so binary compiles
  touch ./static/css/manifest.txt # create empty manifest needed so binary compiles
  # in development mode, frontend changes do not trigger a rebuild of the backend

watch-frontend: install-frontend
  cargo watch -w frontend \
    -s 'just build-dev-frontend'

build-dev-backend: build-dev-frontend
  cargo run

watch-backend:
  mold -run cargo watch \
    --ignore 'logs/*' \
    --ignore 'static/*' \
    --ignore 'frontend/*' \
    --ignore 'content' \
    --ignore 'content/*' \
    --no-vcs-ignores \
    --why \
    -s 'just build-dev-backend'

# runs watch-frontend and watch-backend simultaneously
watch:
 ./watch.sh

reset:
  sqlx database reset

migrate:
  sqlx migrate run

prepare:
  cargo sqlx prepare
