from sys import argv
import shutil

from filter_profile import main as filter_profile
import numpy as np

TARGETS = {
    "dnn": 950000,
    "masstree": 427000,
    "moses": 2710000,
    "silo": 570000,
    "xapian": 4200000,
    "system": 1
}

def read_profile(fname):
    with open(fname) as f:
        lines = [line.strip() for line in f.readlines() if len(line) > 2]
    lines = [line.split(",") for line in lines]
    return lines

def compile_profile(p):
    header = "id,perf0,perf1,harmonicMeanPerf,powerConsumption"
    lines = []
    for idx, config in enumerate(zip(*p)):
        s = ",".join([str(idx)] + [str(entry) for entry in config])
        lines.append(s)
    return "\n".join([header] + lines)


def make_generic_profile(app0, app1, app0_name, app1_name):
    app0_lat_index = app0[0].index("appLatency")
    app1_lat_index = app1[0].index("appLatency")

    app0_lats = np.array([float(i[app0_lat_index]) for i in app0[1:]])
    app1_lats = np.array([float(i[app1_lat_index]) for i in app1[1:]])

    app0_min_lat = min(app0_lats)
    app1_min_lat = min(app1_lats)

    app0_xup = app0_lats / app0_min_lat
    app1_xup = app1_lats / app1_min_lat

    app0_target_xup = TARGETS[app0_name] / app0_min_lat
    app1_target_xup = TARGETS[app1_name] / app1_min_lat

    generic_xup = (app0_xup + app1_xup) / 2

    perf0 = app0_target_xup / generic_xup
    perf1 = app1_target_xup / generic_xup
    hm = 2 / ((1 / perf0) + (1 / perf1))

    app0_pow_index = app0[0].index("powerConsumption")
    app0_pows = np.array([float(i[app0_lat_index]) for i in app0[1:]])
    print(app0_pows)

    app1_pow_index = app1[0].index("powerConsumption")
    app1_pows = np.array([float(i[app1_lat_index]) for i in app1[1:]])
    print(app1_pows)

    pows = app0_pows + app1_pows

    return compile_profile([list(perf0), list(perf1), list(hm), list(pows)])

def main(app0, app1):
    profiles = []
    for app in [app0, app1]:
        filter_profile(f"../profiles/with-hyperthreading/{app}.mt", f"../profiles/with-hyperthreading/{app}.kt")
        p = read_profile("./sys_mt_filtered")
        if profiles:
            assert(len(p) == len(profiles[-1]))
        profiles.append(p)

    app0 = app0.replace("-new", "")
    app1 = app1.replace("-new", "")

    generic_profile = make_generic_profile(profiles[0], profiles[1], app0, app1)

    with open(f"./profiles-temp/multi-{app0}-{app1}-generic.mt", "w") as f:
        f.write(generic_profile)
    shutil.copy("sys_kt_filtered", f"./profiles-temp/multi-{app0}-{app1}-generic.kt")

if __name__ == "__main__":
    main(argv[1], argv[2])