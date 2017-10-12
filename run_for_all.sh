#! /usr/bin/env bash

function run_all {
  for TOML_DIR in $(find . -type f | grep "Cargo.toml$" | sed "s/Cargo.toml$//g")
  do
    (cd $TOML_DIR && (cargo $@)) || return 1
  done
  return 0
}

if [[ ! "$PWD" =~ "stockpile" ]]; then
  echo "Please run $0 from the stockpile directory"
  exit 1
fi

run_all $@

