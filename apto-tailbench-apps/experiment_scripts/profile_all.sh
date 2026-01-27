#!/bin/bash

applications=("silo")
log_file="profilingSilo.log"

echo "Profiling times (with hyperthreading)" > "$log_file"

cargo build --release --bin main

for app in "${applications[@]}"; do
    #rm -rf knobtable measuretable
    echo "Profiling $app..." | tee -a "$log_file"
    { sudo RUST_LOG=info RUST_BACKTRACE=1 PROFILE=1000 ./target/release/main --warmup-time 10 profile "$app"; } 2>> "$log_file"
    #mv knobtable profiles/with-hyperthreading/${app}.kt
    #mv measuretable profiles/with-hyperthreading/${app}.mt
done

