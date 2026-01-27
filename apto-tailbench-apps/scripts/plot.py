import matplotlib.pyplot as plt
import numpy as np
from sys import argv
from pathlib import Path
from collections import deque

def running_mean(x, N):
    result = []
    window = deque()
    current_sum = 0
    for val in x:
        if len(window) > N:
            removed_val = window.popleft()
            current_sum -= removed_val
        window.append(val)
        current_sum += val
        result.append(current_sum / len(window))
    return result


def mape(actual, pred):
    actual, pred = np.array(actual), np.array(pred)
    return np.mean(np.abs((actual - pred) / actual)) * 100


def parse_measures(line):
    measures = {}
    for entry in line:
        name, value = entry.strip().split(":")
        try:
            value = float(value)
        except ValueError:
            value = value
        measures[name] = value
    return measures


def read_controller_lines(fname):
    with open(fname) as f:
        lines = [line.strip() for line in f.readlines() if len(line) > 5 and "tag" in line]

    tag_lines = []
    for line in lines:
        parsed_tag_line = parse_measures(line.split("]")[1].split(","))
        tag_lines.append(parsed_tag_line)

    for idx in range(0, len(tag_lines), 2):
        if tag_lines[idx]["tag"] == 0:
            first = tag_lines[idx]
            second = tag_lines[idx + 1]
        else:
            first = tag_lines[idx + 1]
            second = tag_lines[idx]
        print(f"{first['tag']},{first['change']},{second['tag']},{second['change']}")


def read_logs(fname, instance_id=0):
    with open(fname) as f:
        lines = [line.strip() for line in f.readlines() if len(line) > 5]

    log = []
    for line in lines:
        if "Initialized" in line or "apto::optimize]" not in line or "performance" not in line:
            continue
        line = line.split("apto::optimize]")[1].strip().split(",")
        measures = parse_measures(line)
        if measures["instance"] != instance_id:
            continue
        log.append(measures)

    return log

def multi_comparison_time_series(prefix):
    # # Search Application
    TARGETS = {"appLatency": 6000000.0, "powerConsumption": 100000000.0}
    WINDOW_SIZE = 200
    CALC_POINT = 1000

    MEASURES_TO_PLOT = [
        ("uncoreFrequency", False, 0, ""),
        ("utilizedCoreFrequency", False, 0, ""),
        ("numCores", False, 0, "#Cores"),
        ("powerConsumption", True, 0, "Power\nConsumption"),
        ("appLatency", True, 0, "Latency (ns)")
    ]
    MAKE_AVERAGES = ["performance", "searchedDocs"]
    YLIMS = {}

    FILES_TO_PLOT = [
        ("output-0", "Monolithic"),
        # ("output-power", "Monolithic (power)"),
        ("output-power-min-latency", "Monolithic (power-min-lat)"),
    ]

    data = {}
    for fname, _ in FILES_TO_PLOT:
        data[fname] = (read_logs(prefix + "/" + fname), read_logs(prefix + "/" + fname, 1))

    errors = {}
    _, axs = plt.subplots(len(MEASURES_TO_PLOT), 1, figsize=(15, 10))
    for (ax, (measure_name, plot_meaned, try_module, measure_label)) in zip(axs, MEASURES_TO_PLOT):
        if not measure_label:
            measure_label = measure_name
        ax.set_ylabel(measure_label)

        nr_samples = float("inf")
        min_y, max_y = float("inf"), float("-inf")
        for fname, label in FILES_TO_PLOT:
            if measure_name in data[fname][try_module][0]:
                dp = data[fname][try_module]
            else:
                dp = data[fname][1 - try_module]  # We're assuming that we only have modules 0 and 1

            measure_values = [e[measure_name] for e in dp]
            if plot_meaned:
                measure_values = running_mean(measure_values, WINDOW_SIZE)
            ax.plot(measure_values, label=label)
            nr_samples = min(nr_samples, len(measure_values))
            min_y, max_y = min(min_y, min(measure_values[CALC_POINT:])), max(max_y, max(measure_values[CALC_POINT:]))

            if measure_name in TARGETS:
                key = measure_name + "(MAPE)"
                measurement_subset = measure_values[CALC_POINT:]
                execution_error = round(mape(np.zeros(len(measurement_subset)) + TARGETS[measure_name], measurement_subset), 3)
                errors[key] = errors.get(key, []) + [f"{label}: {execution_error}"]
            elif measure_name in MAKE_AVERAGES:
                key = measure_name + "(MEAN)"
                measurement_subset = measure_values[CALC_POINT:]
                average_value = round(np.mean(measurement_subset), 3)
                errors[key] = errors.get(key, []) + [f"{label}: {average_value}"]

        if measure_name in TARGETS:
            values = np.zeros(nr_samples) + TARGETS[measure_name]
            ax.plot(values, color="grey", alpha=0.3)

        if measure_name in YLIMS:
            ax.set_ylim(*YLIMS[measure_name])
        else:
            if measure_name in TARGETS:
                ax.set_ylim(min(0.8 * min_y, TARGETS[measure_name]), max(1.2 * max_y, TARGETS[measure_name]))
            else:
                                                                                                                                                                                                                                                                                                                                                                                                                                               ax.set_ylim(0.8 * min_y, 1.2 * max_y)

        ax.set_xlim(0, nr_samples)

    plt.legend(bbox_to_anchor=(0.0,  -0.5, 1., .102), loc=3,
               ncol=5, mode="expand", borderaxespad=0., fontsize=10,
               handletextpad=0.1)

    title_content = []
    for key, value in errors.items():
        error_string = f"{key}: " + ", ".join(value)
        title_content.append(error_string)
    plt.suptitle("\n".join(title_content))

    plt.savefig(prefix + "/results.png", bbox_inches="tight")

    return title_content


def calculate_reward(data, fname, obj_details, constraint_details, refname, calc_point):
    obj_name, try_module, invert, _ = obj_details
    cname, cval = constraint_details
    penalty = 0.5
    if invert:
        penalty = 1.5

    dp, refvals = data[fname][try_module], data[refname][try_module]

    obj_mean = np.mean(np.array([e[obj_name] for e in dp])[calc_point:])
    cons_values = np.array(running_mean([e[cname] for e in dp], 20)[calc_point:])
    error = mape(np.zeros(len(cons_values)) + cval, cons_values)
    exec_reward = obj_mean
    if error > 20:
        exec_reward *= penalty

    ref_obj_mean = np.mean(np.array([e[obj_name] for e in refvals])[calc_point:])
    ref_cons_values = np.array(running_mean([e[cname] for e in refvals], 20))[calc_point:]
    error = mape(np.zeros(len(ref_cons_values)) + cval, ref_cons_values)
    ref_reward = ref_obj_mean
    if error > 20:
        ref_reward *= penalty

    relative_reward = exec_reward / ref_reward
    if invert:
        relative_reward = ref_reward / exec_reward

    return relative_reward


def multi_bar_chart(prefix):
    APPLICATIONS = {
        "aes": {
            "app": ("blockStrength", 0, False, "Block Strength"),
            "sys": ("energyDelta", 1, True, "Energy Per Iteration"),
            "constraint": ("latency", 0.018),
            "files": [
                ("app", "application-only"),
                ("sys", "system-only"),
                ("mm", "mm-static"),
                ("mm-both-dynamic-no-reversal", "mm-both-dynamic")
            ],
            "reference": {
                "sys": "sys",
                "app": "app"
            }
        },
        "search": {
            "app": ("searchedDocs", 0, False, "Searched Documents"),
            "sys": ("performance", 1, False, "Performance"),
            "constraint": ("energyDelta", 500000.0),
            "files": [
                ("app", "application-only"),
                ("sys", "system-only"),
                ("mm", "mm-static"),
                ("mm-both-dynamic-no-reversal", "mm-both-dynamic")
            ],
            "reference": {
                "sys": "sys",
                "app": "app"
            }
        }
    }
    CALC_POINT = 200

    data = {}
    for app in APPLICATIONS.keys():
        if app not in data:
            data[app] = {}
        for fname, _ in APPLICATIONS[app]["files"]:
            data[app][fname] = (read_logs(f"{prefix}/{fname}-{app}"), read_logs(f"{prefix}/{fname}-{app}", 1))

    _, axs = plt.subplots(2, 1)
    app_names = sorted(APPLICATIONS.keys())
    for ax, obj_type in zip(axs, ["app", "sys"]):
        bars = [[] for _ in range(len(app_names))] # Each cluster represented as a list in this list of lists
        labels = []
        for bars_cluster, app in zip(bars, app_names):
            app_data = data[app]

            for fname, label in APPLICATIONS[app]["files"]:
                reward = calculate_reward(app_data, fname, APPLICATIONS[app][obj_type], APPLICATIONS[app]["constraint"],
                                          APPLICATIONS[app]["reference"][obj_type], CALC_POINT)

                bars_cluster.append(reward)
                if label not in labels:
                    labels.append(label)

        for offset, app_bars in enumerate(zip(*bars)):
            print(obj_type, labels[offset], *app_bars)
            x_vals = np.arange(0, len(app_bars)) * 4 + 0.5 * offset
            ax.bar(x_vals, app_bars, label=labels[offset], width=0.5)

        x_vals = np.arange(0, len(app_bars)) * 4 + 0.75
        ax.set_xticks(x_vals)
        xtick_labels = []
        for app in app_names:
            xtick_labels.append(f"{app}\n{APPLICATIONS[app][obj_type][-1]}")
        ax.set_xticklabels(xtick_labels)
        ax.set_ylabel(obj_type)
        ax.legend()

    plt.tight_layout()
    plt.savefig("bars.png")


def extract_controller_logs(prefix):
    for fname in ["mm-both-dynamic", "mm-both-dynamic-no-reversal"]:
        with open(prefix + f"/{fname}") as f:
            lines = [line.strip().split("]")[1].split("::")[1].strip() for line in f.readlines() if "inaccuracy" in line]
            # lines = [line.strip().split("]")[1].strip() for line in f.readlines() if ("controller::optimizing_controller" in line and "tag" in line) and ("Multiplier" not in line)]
        records = [parse_measures(line.split(',')) for line in lines]
        measure_names = ["tag"] + sorted([k for k in records[0].keys() if k != "tag"])
        tag_records = {}
        for record in records:
            if record["tag"] not in tag_records:
                tag_records[int(record["tag"])] = []
            tag_records[record["tag"]].append(record)
        with open(f"{prefix}/{fname}.csv", "w") as f:
            f.write(",".join(measure_names) + "\n")
            for (tag, records) in tag_records.items():
                for record in records:
                    values = [str(record[name]) for name in measure_names]
                    f.write(",".join(values) + "\n")
                for _ in range(8):
                    f.write("\n")


def main(prefix):
    multi_comparison_time_series(prefix)
    # multi_bar_chart(prefix)



if __name__ == "__main__":
    if len(argv) == 2:
        main(argv[1])
    else:
        main("./")
