import re

pattern = re.compile("[a-zA-Z_]+:\s*\-{0,1}[0-9.]+")

# i0 = "[2022-05-12T22:45:06Z INFO  apto::optimize] instance:1,performance:4390.181797428231,energyDelta:14764794,appLatency:1653095,numCores:none,powerConsumption:38999878.18872498,latency:0.000227781,energy:none,windowLatency:0.378585644,iteration:399,utilizedCoreFrequency:2000,uncoreFrequency:12"
# print(pattern.findall(i0))
# for e in pattern.findall(i0):
#     k = (e.strip().split(":")[0])
#     print(f"{k}: float")

# i1 = "[2022-05-13T21:16:14Z INFO  OptimizingController::controller::optimizing_controller] tag: 0, measured: 3056947.0201005023, workload: 0.0000003271237589086916, derivative: 0.39027597536193936, xup: 1.3317847086292804, sched_xup: 1.3317847086292804"
# for e in pattern.findall(i1):
#     k = (e.strip().split(":")[0])
#     print(f"{k}: float")

# i2 = "[2022-05-13T21:16:14Z INFO  apto::optimize] Obtained new schedule (4, 9, 5) for window average 3056947.0201005023 (instance 1)"
# print(pattern.findall(i2))

# i3 = "[2022-05-13T21:16:15Z INFO  OptimizingController::controller::optimizing_controller] Multiplier Adaptatoin :: tag: 0, workload: 0.00000015515546336867472, pole: 0.95, derivative: -1.0425913651517824, inaccuracy: 1.0425913651517824, e: -4333550.199999999, eo: 1193052.9798994977, eoo: 0"
# for e in pattern.findall(i3):
#     k = (e.strip().split(":")[0])
#     print(f"{k}: float")


GOAL_PATTERN = re.compile("\(instance (\d+)\) to \w+\((\w+)\) such that (\w+) == ([0-9.]+) with window (\d+)$")
sample = "[2022-05-13T21:16:04Z INFO  apto::optimize] Initialized Apto (instance 0) to minimize(numCores) such that appLatency == 4250000 with window 200"
print(GOAL_PATTERN.findall(sample))
