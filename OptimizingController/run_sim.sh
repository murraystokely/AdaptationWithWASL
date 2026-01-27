#!/bin/bash

set -e

cargo build --release

# Battery Saver
for i in 1 2 3
do
    cargo run --release -- profiles/object_detector.measuretable proteus-outputs/battery_saver performance 30 40 max 'quality'
done
cargo run --release -- profiles/object_detector.measuretable proteus-outputs/battery_saver performance 30 40 max 'quality' > bs
echo "Done battery saver ========================="

# Object Detector
for i in 1 2 3
do
    cargo run --release -- profiles/object_detector.measuretable proteus-outputs/object_detector performance 20 40 max 'quality'
done
cargo run --release -- profiles/object_detector.measuretable proteus-outputs/object_detector performance 20 40 max 'quality' > obj
echo "Done object detector ========================="

# SAR
for i in 1 2 3
do
    cargo run --release -- profiles/radar.measuretable proteus-outputs/radar quality 0.5 20 max 'performance'
done
cargo run --release -- profiles/radar.measuretable proteus-outputs/radar quality 0.5 20 max 'performance' > radar
echo "Done SAR ========================="
# SAR 2
for i in 1 2 3
do
    cargo run --release -- profiles/radar.measuretable proteus-outputs/radar2 performance 60.0 20 max 'quality'
done
cargo run --release -- profiles/radar.measuretable proteus-outputs/radar2 performance 60.0 20 max 'quality' >> radar
echo "Done SAR 2 ========================="


# AES max(performance) such that powerConsumption == 1500000.0
for i in 1 2 3
do
    cargo run --release -- profiles/aes.measuretable proteus-outputs/aes powerConsumption 1500000.0 32 max 'performance'
done
cargo run --release -- profiles/aes.measuretable proteus-outputs/aes powerConsumption 1500000.0 32 max 'performance' > aes
echo "Done AES ========================="
# AES 2 : max(performance/powerConsumption) such that blockStrength == 256.0
for i in 1 2 3
do
    cargo run --release -- profiles/aes.measuretable proteus-outputs/aes2 blockStrength 256.0 32 max 'performance / powerConsumption'
done
cargo run --release -- profiles/aes.measuretable proteus-outputs/aes2 blockStrength 256.0 32 max 'performance / powerConsumption' >> aes
echo "Done AES 2 ========================="


# SOA
for i in 1 2 3
do
    cargo run --release -- profiles/soa.measuretable proteus-outputs/soa responseTime 0.5 40 max 'reliability'
done
cargo run --release -- profiles/soa.measuretable proteus-outputs/soa responseTime 0.5 40 max 'reliability' > soa
echo "Done SOA ========================="
# SOA 2
for i in 1 2 3
do
    cargo run --release -- profiles/soa.measuretable proteus-outputs/soa2 reliability 0.6 40 min 'cost'
done
cargo run --release -- profiles/soa.measuretable proteus-outputs/soa2 reliability 0.6 40 max 'cost' >> soa
echo "Done SOA2 ========================="


# Search Engine
for i in 1 2 3
do
    cargo run --release -- profiles/search_engine.measuretable proteus-outputs/search_engine performance 18.0 40 min 'powerConsumption'
done
cargo run --release -- profiles/search_engine.measuretable proteus-outputs/search_engine performance 18.0 40 min 'powerConsumption' > search_engine
echo "Done Search Engine ========================="
