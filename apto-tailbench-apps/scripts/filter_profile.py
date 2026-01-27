from sys import argv


def write_profile(measure_table, knob_table, prefix):
    with open(prefix + "_mt_filtered", "w") as f:
        f.write("\n".join(measure_table))

    with open(prefix + "_kt_filtered", "w") as f:
        f.write("\n".join(knob_table))


def make_app_profile(mt_lines, kt_lines):
    # Implement filter function
    knob_names = kt_lines[0].split(',')
    filtered_knob_idx_names = [(idx, k) for (idx, k) in enumerate(knob_names)
                               if k in ["id", "utilizedPhysicalCores"]]
    filtered_knob_names = [name for (_, name) in filtered_knob_idx_names]
    filtered_indices = [idx for (idx, _) in filtered_knob_idx_names]
    filtered_configs = []
    indices = []
    for line in kt_lines[1:]:
        config = line.split(',')
        if (
                config[knob_names.index("uncoreFrequency")] != "24" or
                config[knob_names.index("utilizedCoreFrequency")] != "2000"
        ):
            continue

        config = [c for (idx, c) in enumerate(config) if idx in filtered_indices]

        indices.append(int(config[0]))
        config[0] = str(len(filtered_configs))
        filtered_configs.append(config)

    measure_names = mt_lines[0]
    filtered_measure_table = []
    for idx in indices:
        line = mt_lines[idx + 1].split(',')
        line[0] = str(len(filtered_measure_table))
        filtered_measure_table.append(line)

    measure_table = [measure_names]
    knob_table = [",".join(filtered_knob_names)]
    for config, line in zip(filtered_configs, filtered_measure_table):
        knob_table.append(",".join(config))
        measure_table.append(",".join(line))

    return measure_table, knob_table


def make_sys_profile(mt_lines, kt_lines):
    # Implement filter function
    knob_names = kt_lines[0].split(',')
    filtered_knob_idx_names = [(idx, k) for (idx, k) in enumerate(knob_names)
                               if k in ["id", "utilizedCoreFrequency", "uncoreFrequency"]]
    filtered_knob_names = [name for (_, name) in filtered_knob_idx_names]
    filtered_indices = [idx for (idx, _) in filtered_knob_idx_names]
    filtered_configs = []
    indices = []
    for line in kt_lines[1:]:
        config = line.split(',')
        if config[knob_names.index("utilizedPhysicalCores")] != "2":
            continue

        config = [c for (idx, c) in enumerate(config) if idx in filtered_indices]

        indices.append(int(config[0]))
        config[0] = str(len(filtered_configs))
        filtered_configs.append(config)

    measure_names = mt_lines[0]
    filtered_measure_table = []
    for idx in indices:
        line = mt_lines[idx + 1].split(',')
        line[0] = str(len(filtered_measure_table))
        filtered_measure_table.append(line)

    measure_table = [measure_names]
    knob_table = [",".join(filtered_knob_names)]
    for config, line in zip(filtered_configs, filtered_measure_table):
        knob_table.append(",".join(config))
        measure_table.append(",".join(line))

    return measure_table, knob_table


def get_ranges(name, application, system):
    for profile in [application, system]:
        measure_idx = profile[0].split(",").index(name)
        measure_lines = profile[1:]
        values = []
        for line in measure_lines:
            value = float(line.split(",")[measure_idx])
            values.append(value)


def main(mt_fname, kt_fname):
    with open(mt_fname) as f:
        mt_lines = [line.strip() for line in f.readlines()]

    with open(kt_fname) as f:
        kt_lines = [line.strip() for line in f.readlines()]

    app_measure_table, app_knob_table = make_app_profile(mt_lines, kt_lines)
    write_profile(app_measure_table, app_knob_table, "app")

    sys_measure_table, sys_knob_table = make_sys_profile(mt_lines, kt_lines)
    write_profile(sys_measure_table, sys_knob_table, "sys")

    get_ranges("performance", app_measure_table, sys_measure_table)


if __name__ == "__main__":
    main(argv[1], argv[2])
