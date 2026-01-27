from pathlib import Path
from analysis.exec_logs import ExecutionLog
from utils import CALC_POINTS, COLORS, verify_data_multi_application
import matplotlib.pyplot as plt
import numpy as np
from scipy.stats.mstats import gmean


def compile_data(adapt_type, ptile):
    direc = Path(f"./long-data-collection/multi-application/{adapt_type}")
    applications = ["xapian", "moses", "masstree", "silo", "dnn"]

    data = []
    labels = []

    for idx0, app0 in enumerate(applications):
        for app1 in applications[idx0 + 1:]:
            pair_data = []
            for variant in ["multimodule", "dynamic"]:
                path = direc.joinpath(f"{app0}-{app1}-{variant}")
                log = ExecutionLog(path)
                variant_data = log.latency_percentile(ptile, [CALC_POINTS[app0], CALC_POINTS[app1], CALC_POINTS["system"]])[:2]
                print(variant_data)
                pair_data.append(variant_data)
            data.append(pair_data)
            labels.append(f"{app0}-{app1}")

    print(list(zip(labels, data)))

    return labels, data


def make_chart(data, outfile, ylabel):
    _, axs = plt.subplots(1, 11, figsize=(18, 4))

    xups = []
    for d, ax in zip(data, axs):
        app0, app1 = d[0].split("-")
        multimodule = np.array(d[1][0])
        dynamic = np.array(d[1][1]) / multimodule
        multimodule /= multimodule


        ax.bar([0, 1.1], multimodule, width=0.5, label="Multimodule", color=COLORS["multimodule"])
        ax.bar([0.5, 1.6], dynamic, width=0.5, label="Dynamic", color=COLORS["dynamic"])

        xups += list(dynamic)

        ax.set_xticks([0.25, 1.35])
        ax.set_xticklabels([app0, app1], fontsize=8)

    axs[-1].bar([0], [1], width=0.5, label="Multimodule", color=COLORS["multimodule"])
    axs[-1].bar([0.5], [gmean(xups)], width=0.5, label="Dynamic", color=COLORS["dynamic"])
    axs[-1].set_xticks([0.25])
    axs[-1].set_xticklabels(["Geometric\nMean"])

    axs[0].set_ylabel(ylabel)

    axs[0].legend(bbox_to_anchor=(0.0,  -0.25, 15.0, 1.102), loc=3,
                   ncol=5, mode="expand", borderaxespad=0., fontsize=10,
                   handletextpad=0.1)

    plt.tight_layout()

    plt.savefig(outfile)

    plt.cla()
    plt.clf()
    plt.close("all")


def main():
    from current_data import control_multi_95 as control_95
    from current_data import control_multi_99 as control_99
    make_chart(control_95, "charts/multi-control-95ptile.png", "95th Percentile")
    make_chart(control_99, "charts/multi-control-99ptile.png", "99th Percentile")

    from current_data import learning_multi_95 as learning_95
    from current_data import learning_multi_99 as learning_99
    make_chart(learning_95, "charts/multi-learning-95ptile.png", "95th Percentile")
    make_chart(learning_99, "charts/multi-learning-99ptile.png", "99th Percentile")

    from current_data import pi_multi_95 as pi_95
    from current_data import pi_multi_99 as pi_99
    make_chart(pi_95, "charts/multi-pi-95ptile.png", "95th Percentile")
    make_chart(pi_99, "charts/multi-pi-99ptile.png", "99th Percentile")


if __name__ == "__main__":
    main()
