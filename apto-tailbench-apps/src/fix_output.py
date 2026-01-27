from sys import argv


def main(fname_src, fname_dst):
    with open(fname_src) as f:
        lines = [l.strip() for l in f.readlines()]

    final_lines = []
    current_lines = set()
    for entry in enumerate(lines):
        if "REPLACE" in entry[1]:
            line = entry[1]
            replacement = line.split("REPLACE:")[1].strip().split(",")

            new_working_set = set()
            for (i, line) in current_lines:
                if replacement[0] in line:
                    line = line.replace("powerConsumption:none", replacement[1])
                    line = line.replace("energyDelta:none", replacement[2])
                    line = line.replace("windowLatency:none", replacement[3])
                    final_lines.append((i, line))
                else:
                    new_working_set.add((i, line))

            current_lines = new_working_set
            continue
        if "none" not in entry[1]:
            final_lines.append(entry)
        else:
            current_lines.add(entry)

    final_lines.sort(key=lambda e: e[0])
    final_lines = [l[1] for l in final_lines]

    with open(fname_dst, "w") as f:
        f.write("\n".join(final_lines))


if __name__ == "__main__":
    main(argv[1], argv[2])
