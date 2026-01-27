from pathlib import Path
from analysis.exec_logs import ExecutionLog
from utils import CALC_POINTS, COLORS
import numpy as np
import matplotlib.pyplot as plt
from scipy.stats.mstats import gmean


def make_chart(nonadaptive_all, adaptive_all, outfile, ylabel):
    _, axs = plt.subplots(1, 11, figsize=(18, 4))

    xups = [[], [], [], []]

    for nonadaptive, adaptive, ax in zip(nonadaptive_all, adaptive_all, axs):
        app0, app1 = nonadaptive[0].split("-")

        standalone, colocated = nonadaptive[1]
        multimodule, dynamic = adaptive[1]

        standalone = np.array(standalone)
        colocated = np.array(colocated) / standalone
        multimodule = np.array(multimodule) / standalone
        dynamic = np.array(dynamic) / standalone
        standalone /= standalone

        ax.bar([0, 2.1], standalone, width=0.5, label="Standalone", color=COLORS["standalone"])
        ax.bar([0.5, 2.6], colocated, width=0.5, label="Colocated", color=COLORS["colocated"])
        ax.bar([1, 3.1], multimodule, width=0.5, label="Multimodule", color=COLORS["multimodule"])
        ax.bar([1.5, 3.6], dynamic, width=0.5, label="Dynamic", color=COLORS["dynamic"])

        xups[0] += list(standalone)
        xups[1] += list(colocated)
        xups[2] += list(multimodule)
        xups[3] += list(dynamic)

        ax.set_xticks([0.75, 2.85])
        ax.set_xticklabels([app0, app1])

    xups = [gmean(x) for x in xups]
    axs[-1].bar([0], [xups[0]], width=0.5, label="Standalone", color=COLORS["standalone"])
    axs[-1].bar([0.5], [xups[1]], width=0.5, label="Colocated", color=COLORS["colocated"])
    axs[-1].bar([1], [xups[2]], width=0.5, label="Multimodule", color=COLORS["multimodule"])
    axs[-1].bar([1.5], [xups[3]], width=0.5, label="Dynamic", color=COLORS["dynamic"])
    axs[-1].set_xticks([0.75])
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
    from current_data import non_adaptive_95, control_multi_95, non_adaptive_99, control_multi_99

    make_chart(non_adaptive_95, control_multi_95, "charts/combined_bars_95ptile.png", "95th Percentile")

    make_chart(non_adaptive_99, control_multi_99, "charts/combined_bars_99ptile.png", "99th Percentile")


if __name__ == "__main__":
    main()
