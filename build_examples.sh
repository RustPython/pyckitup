#!/bin/bash

cd "$(dirname "$0")"

for example in examples/*.py; do
  name=$(basename "$example" .py)
  if [[ $name = common ]] || [[ $name = __init__ ]]; then
    continue
  fi
  target/debug/pyckitup build "$example"
  mv build/index.html build/"$name".html
done
