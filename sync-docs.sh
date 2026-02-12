#!/bin/bash
# Sync source docs into site-docs for mkdocs
cd "$(dirname "$0")"
cp README.md site-docs/index.md
cp -r docs/* site-docs/docs/
cp -r spec/* site-docs/spec/
/home/reza/.local/bin/mkdocs build
echo "Docs synced and built"
