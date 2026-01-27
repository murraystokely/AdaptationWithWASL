from pathlib import Path
from analysis.exec_logs import ExecutionLog
from utils import CALC_POINTS, COLORS, verify_data_multi_application
import numpy as np
import matplotlib.pyplot as plt
from scipy.stats.mstats import gmean


def compile_data(ptile):
    exec_logs_cache = {}
    direc = Path("./long-data-collection/nonadaptive/")
    applications = ["xapian", "moses", "masstree", "silo", "dnn"]

    data = []
    labels = []
    for idx0, app0 in enumerate(applications):
        if app0 not in exec_logs_cache:
            exec_logs_cache[app0] = ExecutionLog(direc.joinpath(f"{app0}"))
        app0_log = exec_logs_cache[app0]

        for app1 in applications[idx0 + 1:]:
            if app1 not in exec_logs_cache:
                exec_logs_cache[app1] = ExecutionLog(direc.joinpath(f"{app1}"))
            app1_log = exec_logs_cache[app1]

            combined_name = f"{app0}-{app1}"
            if combined_name not in exec_logs_cache:
                exec_logs_cache[f"{app0}-{app1}"] = ExecutionLog(direc.joinpath(combined_name), True)
            combined_log = exec_logs_cache[combined_name]

            combined_percentile = combined_log.latency_percentile(ptile, [CALC_POINTS[app0], CALC_POINTS[app1]])
            current_combination_data = [
                [app0_log.latency_percentile(ptile, [CALC_POINTS[app0]])[0], app1_log.latency_percentile(ptile, [CALC_POINTS[app0]])[0]],
                [combined_percentile[0], combined_percentile[1]]
            ]
            data.append(current_combination_data)

            labels.append(f"{app0}-{app1}")

            print(f"{labels[-1]},{current_combination_data}")

def make_chart(data, outfile, ylabel):
    _, axs = plt.subplots(1, 11, figsize=(18, 4))

    xups = []
    for d, ax in zip(data, axs):
        app0, app1 = d[0].split("-")
        standalone = np.array(d[1][0])
        colocated = np.array(d[1][1]) / standalone
        standalone /= standalone


        ax.bar([0, 1.1], standalone, width=0.5, label="Standalone", color=COLORS["standalone"])
        ax.bar([0.5, 1.6], colocated, width=0.5, label="Colocated", color=COLORS["colocated"])

        xups += list(colocated)

        ax.set_xticks([0.25, 1.35])
        ax.set_xticklabels([app0, app1], fontsize=8)

    axs[-1].bar([0], [1], width=0.5, label="Standalone", color=COLORS["standalone"])
    axs[-1].bar([0.5], [gmean(xups)], width=0.5, label="Colocated", color=COLORS["colocated"])
    axs[-1].set_xticks([0.25])
    axs[-1].set_xticklabels(["Geometric\nMean"])

    axs[0].legend(bbox_to_anchor=(0.0,  -0.25, 16.0, 1.102), loc=3,
                   ncol=5, mode="expand", borderaxespad=0., fontsize=10,
                   handletextpad=0.1)

    axs[0].set_ylabel(ylabel)

    plt.tight_layout()

    plt.savefig(outfile)

    plt.cla()
    plt.clf()
    plt.close("all")

def main():
    # Cached data from compile_data function
    from current_data import non_adaptive_95, non_adaptive_99
    make_chart(non_adaptive_95, "charts/nonadaptive_95ptile.png", "95th Percentile")
    make_chart(non_adaptive_99, "charts/nonadaptive_99ptile.png", "99th Percentile")


if __name__ == "__main__":
    main()
