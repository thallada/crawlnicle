#!/usr/bin/env just --justfile

build-frontend:
  bun build frontend/index.ts --outdir ./static/js --entry-naming [name]-[hash].[ext]
