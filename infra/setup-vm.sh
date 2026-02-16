#!/usr/bin/env bash
# setup-vm.sh — Run on the AWS bare-metal instance after first SSH login.
# Installs all dependencies, clones repos, builds TailBench and WASL,
# and enables RAPL/MSR/cpufreq access required for experiments.
#
# Usage:  chmod +x setup-vm.sh && sudo ./setup-vm.sh
#
# The script assumes it is run as root (or with sudo).
# Everything is installed under /opt/wasl.

set -euo pipefail

INSTALL_DIR="/opt/wasl"
TAILBENCH_DIR="${INSTALL_DIR}/tailbench"
WASL_DIR="${INSTALL_DIR}/AdaptationWithWASL"

echo "=== [1/8] System packages ==="
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y \
  build-essential gcc g++ cmake make automake autoconf libtool bison swig git \
  libboost-all-dev zlib1g-dev uuid-dev libicu-dev liblzma-dev libreadline-dev libnuma-dev \
  libpng-dev libjpeg-dev libtiff5-dev libgdk-pixbuf2.0-dev \
  libmysqld-dev libaio-dev libjemalloc-dev libdb5.3++-dev libgoogle-perftools-dev \
  openjdk-8-jdk ant doxygen graphviz imagemagick \
  python3 python3-pip \
  pkg-config libssl-dev curl wget \
  msr-tools linux-tools-common linux-tools-generic linux-tools-"$(uname -r)" \
  cpufrequtils \
  nginx

echo "=== [2/8] Enable MSR and RAPL access ==="
modprobe msr
# Ensure msr module loads on boot
echo "msr" >> /etc/modules-load.d/msr.conf

# intel_pstate driver (common on Xeon) must be switched to passive mode
# before the "userspace" governor becomes available
if [ -f /sys/devices/system/cpu/intel_pstate/status ]; then
  echo "passive" > /sys/devices/system/cpu/intel_pstate/status
  echo "Switched intel_pstate to passive mode"
fi

# Set CPU frequency governor to userspace so WASL can control frequencies
for gov_file in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
  if [ -f "$gov_file" ]; then
    echo "userspace" > "$gov_file" 2>/dev/null || true
  fi
done

# Verify governor was applied
CURRENT_GOV=$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo "unknown")
echo "CPU governor on cpu0: ${CURRENT_GOV}"
if [ "$CURRENT_GOV" != "userspace" ]; then
  echo "WARNING: Could not set userspace governor. You may need to add 'intel_pstate=passive' to kernel boot params in /etc/default/grub, then update-grub and reboot."
fi

# Verify RAPL is accessible
if [ -d /sys/class/powercap/intel-rapl ]; then
  echo "RAPL interface detected at /sys/class/powercap/intel-rapl"
else
  echo "WARNING: RAPL powercap interface not found — energymon will fall back to MSR"
fi

# Verify MSR device nodes exist
if [ -c /dev/cpu/0/msr ]; then
  echo "MSR device nodes available"
else
  echo "WARNING: /dev/cpu/0/msr not found after loading msr module"
fi

echo "=== [3/8] Install Rust toolchain ==="
if ! command -v rustup &> /dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
fi
source "$HOME/.cargo/env"
rustup default stable

echo "=== [4/8] Install energymon ==="
mkdir -p "${INSTALL_DIR}"
cd "${INSTALL_DIR}"
if [ ! -d energymon ]; then
  git clone https://github.com/energymon/energymon.git
fi
cd energymon
mkdir -p build && cd build
cmake .. -DDEFAULT=msr
make -j"$(nproc)"
make install
ldconfig

echo "=== [5/8] Clone TailBench (modified) ==="
cd "${INSTALL_DIR}"
if [ ! -d tailbench ]; then
  git clone https://github.com/adaptsyslearn/TailBenchMod.git tailbench
fi

echo "=== [6/8] Build TailBench applications ==="
cd "${TAILBENCH_DIR}"
# Build each application that has a build.sh
for app_dir in xapian masstree img-dnn moses silo sphinx; do
  if [ -d "${app_dir}" ] && [ -f "${app_dir}/build.sh" ]; then
    echo "--- Building ${app_dir} ---"
    cd "${app_dir}"
    chmod +x build.sh
    ./build.sh || echo "WARNING: ${app_dir} build had errors — check manually"
    cd "${TAILBENCH_DIR}"
  fi
done

echo "=== [7/8] Clone and build AdaptationWithWASL ==="
cd "${INSTALL_DIR}"
if [ ! -d AdaptationWithWASL ]; then
  git clone https://github.com/adaptsyslearn/AdaptationWithWASL.git
fi
cd "${WASL_DIR}"

# Build the main binary
cd apto-tailbench-apps
cargo build --release --bin main
echo "WASL binary built at: ${WASL_DIR}/apto-tailbench-apps/target/release/main"

echo "=== [8/8] Build OptimizingController ==="
cd "${WASL_DIR}/OptimizingController"
cargo build --release

echo ""
echo "============================================================"
echo " Setup complete."
echo ""
echo " Hardware verification:"
echo "   MSR:     $([ -c /dev/cpu/0/msr ] && echo 'OK' || echo 'MISSING')"
echo "   RAPL:    $([ -d /sys/class/powercap/intel-rapl ] && echo 'OK' || echo 'MISSING')"
echo "   cpufreq: $([ -f /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor ] && echo 'OK' || echo 'MISSING')"
echo ""
echo " IMPORTANT: Before running real experiments you must:"
echo "  1. Update binary paths in:"
echo "     ${WASL_DIR}/apto-tailbench-apps/src/apps.rs"
echo "     to point to ${TAILBENCH_DIR}/<app>/<binary>"
echo "  2. Rebuild: cd ${WASL_DIR}/apto-tailbench-apps && cargo build --release --bin main"
echo "  3. Download TailBench input datasets (see TailBenchMod README)"
echo "============================================================"
