from pathlib import Path
from analysis.exec_logs import ExecutionLog
from utils import CALC_POINTS, COLORS
import matplotlib.pyplot as plt
from scipy.stats.mstats import gmean
import numpy as np

logs_cache = {}

def compile_data(adapt_type, ptile):
    global logs_cache

    applications = ["xapian", "moses", "masstree", "silo", "dnn"]
    direc = Path(f"./long-data-collection/single-application/{adapt_type}")

    data = []
    labels = []
    for app in applications:
        curr_data = []
        for variant in ["monolithic", "multimodule", "dynamic"]:
            path = direc.joinpath(f"{app}-{variant}")
            if path.name not in logs_cache:
                logs_cache[path.name] = ExecutionLog(path)
            log = logs_cache[path.name]
            curr_data.append(log.latency_percentile(ptile, [CALC_POINTS[app]])[0])
        data.append(curr_data)
        labels.append(app)

        print(app, curr_data)

    print(list(zip(labels, data)))

    return list(zip(labels, data))


def make_chart(data, outfile, ylabel):
    labels = [d[0] for d in data]
    monolithic = [d[1][0] for d in data]
    multimodule = [d[1][1] for d in data]
    dynamic = [d[1][2] for d in data]

    monolithic = np.array(monolithic)

    multimodule = list(np.array(multimodule) / monolithic)
    multimodule.append(gmean(multimodule))

    dynamic = list(np.array(dynamic) / monolithic)
    dynamic.append(gmean(dynamic))

    monolithic /= monolithic
    monolithic = list(monolithic) + [1]

    labels.append("Geometric\nMean")

    plt.bar(np.arange(len(monolithic)) * 2, monolithic,
            width=0.5, label="Monolithic", color=COLORS["monolithic"])
    plt.bar(np.arange(len(multimodule)) * 2 + 0.5, multimodule,
            width=0.5, label="Multimodule", color=COLORS["multimodule"])
    plt.bar(np.arange(len(dynamic)) * 2 + 1, dynamic,
            width=0.5, label="Dynamic", color=COLORS["dynamic"])
    plt.xticks(np.arange(len(multimodule)) * 2 + 0.5, labels)

    plt.ylabel(ylabel)

    plt.legend()

    plt.savefig(outfile)

    plt.cla()
    plt.clf()
    plt.close("all")




def main():
    # from current_data import control_single_95 as control_95
    # from current_data import control_single_99 as control_99
    control_95 = compile_data("control", 95)
    control_99 = compile_data("control", 99)
    make_chart(control_95, "charts/single-control-95ptile-new.png", "95th Percentile")
    make_chart(control_99, "charts/single-control-99ptile-new.png", "99th Percentile")

    learning_95 = [('xapian', [11.677578199999994, 30.547595699999988, 12.285010799999997]), ('moses', [4.997874899999999, 7.366176099999996, 4.551535499999999]), ('masstree', [0.892504, 18.300998199999995, 1.0835484999999971]), ('silo', [7.048814699999985, 4.971961899999995, 4.840416699999998]), ('dnn', [1.6878851999999993, 2.0187011999999998, 1.882721899999996])]
    learning_99 = [('xapian', [22.81584755999999, 72.22702303999982, 27.781850239999912]), ('moses', [8.675516099999989, 16.17715861999988, 7.650063659999993]), ('masstree', [1.6006137600000043, 27.58875588000003, 4.675194200000011]), ('silo', [12.095844080000006, 8.903904580000022, 8.50659930000001]), ('dnn', [2.3794422400000053, 2.76088854, 3.025757280000008])]
    make_chart(learning_95, "charts/single-learning-95ptile-new.png", "95th Percentile")
    make_chart(learning_99, "charts/single-learning-99ptile-new.png", "99th Percentile")

    # from current_data import pi_single_95 as pi_95
    # from current_data import pi_single_99 as pi_99
    pi_95 = compile_data("pi", 95)
    pi_99 = compile_data("pi", 99)
    make_chart(pi_95, "charts/single-pi-95ptile-new.png", "95th Percentile")
    make_chart(pi_99, "charts/single-pi-99ptile-new.png", "99th Percentile")

if __name__ == "__main__":
    main()
