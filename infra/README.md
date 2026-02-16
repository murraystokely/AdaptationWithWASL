# AWS Infrastructure for AdaptationWithWASL

Step-by-step guide to provision an AWS bare-metal EC2 instance, install all software, run experiments, collect results, and stop the instance when done.

Bare metal is **required** because WASL needs direct hardware access to:
- **RAPL energy counters** — `Energymon::new()` panics if unavailable
- **CPU frequency scaling** (`/sys/devices/system/cpu/cpuN/cpufreq/`) — system module knobs
- **MSR registers** (`/dev/cpu/N/msr`) — uncore frequency control

These are not exposed on any virtualized cloud instance (Azure, AWS standard EC2, GCP).

## Prerequisites (local machine)

1. [AWS CLI v2](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html) installed and configured:
   ```bash
   aws configure
   ```
2. An EC2 key pair. Create one if you don't have one:
   ```bash
   aws ec2 create-key-pair \
     --key-name wasl-key \
     --query 'KeyMaterial' \
     --output text > ~/.ssh/wasl-key.pem
   chmod 400 ~/.ssh/wasl-key.pem
   ```

## Step 1 — Deploy the stack

```bash
aws cloudformation deploy \
  --template-file infra/main.yaml \
  --stack-name wasl \
  --parameter-overrides \
      KeyPairName=wasl-key \
      InstanceType=c5.metal \
      VolumeSize=128 \
      AllowSSHFrom=0.0.0.0/0
```

For better security, replace `AllowSSHFrom` with your IP:
```bash
AllowSSHFrom=$(curl -s https://checkip.amazonaws.com)/32
```

Available bare-metal instance types:

| Instance      | vCPUs | RAM    | On-Demand $/hr | Use case                  |
|---------------|-------|--------|-----------------|---------------------------|
| `c5.metal`    | 96    | 192 GB | ~$3.26          | Cheapest — good default   |
| `c5d.metal`   | 96    | 192 GB | ~$4.61          | + 4x900 GB NVMe local SSD |
| `m5.metal`    | 96    | 384 GB | ~$4.61          | More RAM if needed        |

## Step 2 — Get the public IP and SSH in

```bash
# Get outputs from the stack
aws cloudformation describe-stacks \
  --stack-name wasl \
  --query 'Stacks[0].Outputs' \
  --output table

# Or just the IP:
WASL_IP=$(aws cloudformation describe-stacks \
  --stack-name wasl \
  --query 'Stacks[0].Outputs[?OutputKey==`PublicIP`].OutputValue' \
  --output text)

ssh -i ~/.ssh/wasl-key.pem ubuntu@$WASL_IP
```

Note: Bare-metal instances can take **5–10 minutes** to reach `running` state (longer than regular EC2).

## Step 3 — Upload and run the setup script

From your **local machine**:

```bash
scp -i ~/.ssh/wasl-key.pem infra/setup-vm.sh ubuntu@$WASL_IP:~/setup-vm.sh
```

On the **instance**:

```bash
chmod +x ~/setup-vm.sh
sudo ~/setup-vm.sh 2>&1 | tee ~/setup.log
```

This takes 15–30+ minutes. At completion it prints a hardware verification summary:
```
 Hardware verification:
   MSR:     OK
   RAPL:    OK
   cpufreq: OK
```

All three must show `OK` for experiments to work correctly.

## Step 4 — Post-setup: update binary paths

The hardcoded TailBench binary paths in `apps.rs` must point to the VM install paths.

On the **instance**:

```bash
cd /opt/wasl/AdaptationWithWASL/apto-tailbench-apps/src

# Replace the original author's paths with the bare-metal install paths
sed -i 's|/home/vedant/wasl/tailbench|/opt/wasl/tailbench|g' apps.rs
sed -i 's|/home/cc/wasl/wasl-tailbench|/opt/wasl/tailbench|g' apps.rs

# Rebuild after path changes
cd /opt/wasl/AdaptationWithWASL/apto-tailbench-apps
cargo build --release --bin main
```

## Step 5 — Download TailBench input datasets

TailBench applications require input datasets. See the
[TailBenchMod README](https://github.com/adaptsyslearn/TailBenchMod) for download links.
Place them under `/opt/wasl/tailbench/tailbench.inputs/`.

## Step 6 — Run experiments

All experiment commands run from `/opt/wasl/AdaptationWithWASL/apto-tailbench-apps/`.

```bash
cd /opt/wasl/AdaptationWithWASL/apto-tailbench-apps
mkdir -p results

# --- Profile an application ---
sudo RUST_LOG=info PROFILE=1000 \
  ./target/release/main --warmup-time 30 profile xapian

# --- Single app, monolithic (centralized) ---
sudo CONF_TYPE_0=multi CONF_TYPE_1=multi \
  ./target/release/main --exec-time 300 monolithic xapian 5000 \
  > results/monolithic_xapian.txt 2>&1

# --- Single app, multimodule (uncoordinated) ---
sudo CONF_TYPE_0=multi CONF_TYPE_1=multi \
  ./target/release/main --exec-time 300 multimodule xapian 5000 \
  > results/multimodule_xapian.txt 2>&1

# --- Single app, multimodule + WASL ---
sudo CONF_TYPE_0=multi CONF_TYPE_1=multi \
  ADAPT_TYPE=linear ADAPT_INST=0,1 DEV_TARGET=0.1 \
  ./target/release/main --exec-time 300 multimodule xapian 5000 \
  > results/wasl_xapian.txt 2>&1

# --- Two apps, multimodule + WASL ---
sudo CONF_TYPE_0=multi CONF_TYPE_1=multi CONF_TYPE_2=multi \
  ADAPT_TYPE=linear ADAPT_INST=0,1,2 DEV_TARGET=0.1 \
  ./target/release/main --exec-time 300 multimodule xapian 5000 moses 3000 \
  > results/wasl_xapian_moses.txt 2>&1
```

### Running the OptimizingController simulation (no TailBench required)

```bash
cd /opt/wasl/AdaptationWithWASL/OptimizingController
bash run_sim.sh
```

## Step 7 — Copy results to your local machine

From your **local machine**:

```bash
# Copy experiment results
scp -i ~/.ssh/wasl-key.pem -r \
  ubuntu@$WASL_IP:/opt/wasl/AdaptationWithWASL/apto-tailbench-apps/results/ \
  ./results/

# Copy OptimizingController simulation outputs
mkdir -p oc-results
for f in bs obj radar aes soa search_engine; do
  scp -i ~/.ssh/wasl-key.pem \
    ubuntu@$WASL_IP:/opt/wasl/AdaptationWithWASL/OptimizingController/$f \
    ./oc-results/
done

# Or grab everything at once
scp -i ~/.ssh/wasl-key.pem -r ubuntu@$WASL_IP:/opt/wasl/ ./wasl-full-backup/
```

## Step 8 — Stop the instance (pause billing)

Stopping a bare-metal instance halts compute charges while preserving the EBS volume and all data.

```bash
# Get the instance ID
INSTANCE_ID=$(aws cloudformation describe-stacks \
  --stack-name wasl \
  --query 'Stacks[0].Outputs[?OutputKey==`InstanceId`].OutputValue' \
  --output text)

# Stop the instance
aws ec2 stop-instances --instance-ids $INSTANCE_ID
```

To **restart** later:

```bash
aws ec2 start-instances --instance-ids $INSTANCE_ID

# Wait for it to be running (bare metal can take 5-10 min)
aws ec2 wait instance-running --instance-ids $INSTANCE_ID

# Get the new public IP (it changes on restart)
WASL_IP=$(aws ec2 describe-instances \
  --instance-ids $INSTANCE_ID \
  --query 'Reservations[0].Instances[0].PublicIpAddress' \
  --output text)
echo "SSH: ssh -i ~/.ssh/wasl-key.pem ubuntu@$WASL_IP"
```

After restart, re-apply the CPU governor setting (it doesn't persist across reboots):
```bash
sudo bash -c 'for f in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do echo userspace > "$f"; done'
```

## Step 9 — Tear down everything (when fully done)

This **permanently deletes** the instance, EBS volume, VPC, and all data:

```bash
aws cloudformation delete-stack --stack-name wasl
```

## Cost notes

- **Stopped instances** incur only EBS storage costs (~$10/month for 128 GB gp3).
- **Running instances** incur compute charges: ~$3.26/hr (`c5.metal`), ~$4.61/hr (`m5.metal`).
- **Always stop the instance when not actively running experiments.**
- For extended experiment sessions, consider [Spot Instances](https://aws.amazon.com/ec2/spot/) — `c5.metal` spot pricing is often ~$1.00–1.30/hr (60–70% savings), but instances can be interrupted with 2 min notice.

## Troubleshooting

| Issue | Fix |
|-------|-----|
| Stack creation fails — capacity | Bare-metal instances have limited availability. Try a different AZ or region: add `AvailabilityZone` override or change region. |
| Instance stuck in `pending` | Bare-metal launch takes 5–10 min. Wait longer and check: `aws ec2 describe-instances --instance-ids $INSTANCE_ID --query 'Reservations[0].Instances[0].State.Name'` |
| MSR shows `MISSING` | Run `sudo modprobe msr` manually. If still missing, check `dmesg` for errors. |
| cpufreq shows `MISSING` | The `intel_pstate` driver may need to be switched to passive mode: add `intel_pstate=passive` to kernel boot params in `/etc/default/grub`, then `update-grub && reboot`. |
| RAPL shows `MISSING` | Load the powercap module: `sudo modprobe intel_rapl_common`. The energymon MSR backend will still work without the powercap interface. |
| TailBench app build fails | Check that all system packages installed correctly. Some apps (sphinx, moses) have complex build chains — see individual app READMEs in TailBenchMod. |
| `linux-tools-$(uname -r)` not found | The exact kernel tools package may differ. Run `apt search linux-tools | grep $(uname -r)` or install `linux-tools-aws` instead. |
