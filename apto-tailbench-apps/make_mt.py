import re
import sys
import csv

def parse_log_file(input_file, output_file):
    # Define regex pattern to extract valid log lines
    pattern = re.compile(r"instance:(?P<id>\d+),latency:(?P<latency>[\d\.]+),performance:(?P<performance>[\d\.]+),"
                         r"powerConsumption:(?P<powerConsumption>[\d\.]+),iteration:(?P<iteration>[\d\.]+),"
                         r"numCores:(?P<numCores>[\d\.]+),appLatency:(?P<appLatency>[\d\.]+),"
                         r"windowLatency:(?P<windowLatency>[\d\.]+),energyDelta:(?P<energyDelta>[\d\.]+),"
                         r"energy:(?P<energy>[\d\.]+|none),utilizedCoreFrequency")

    extracted_data = []
    
    with open(input_file, "r") as infile:
        for line in infile:
            match = pattern.search(line)
            if match:
                data = match.groupdict()
                data["energy"] = 0 if data["energy"] == "none" else float(data["energy"])
                extracted_data.append(data)
    
    # Define output CSV headers
    headers = ["id", "appLatency", "energy", "energyDelta", "iteration", "latency", "numCores", "performance", "powerConsumption", "windowLatency"]
    
    # Write to .mt file
    with open(output_file, "w", newline="") as outfile:
        writer = csv.DictWriter(outfile, fieldnames=headers)
        writer.writeheader()
        for row in extracted_data:
            writer.writerow({
                "id": row["id"],
                "appLatency": row["appLatency"],
                "energy": row["energy"],
                "energyDelta": row["energyDelta"],
                "iteration": row["iteration"],
                "latency": row["latency"],
                "numCores": row["numCores"],
                "performance": row["performance"],
                "powerConsumption": row["powerConsumption"],
                "windowLatency": row["windowLatency"]
            })

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python script.py <input_file> <output_file>")
        sys.exit(1)
    
    parse_log_file(sys.argv[1], sys.argv[2])
