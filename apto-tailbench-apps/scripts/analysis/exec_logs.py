from typing import Optional, List, Dict
from dataclasses import dataclass, field
from pathlib import Path
import re
import numpy as np
from .utils import running_mean, mape


ITERATION_LOG_PATTERN = re.compile("[a-zA-Z_]+:\s*\-?[0-9.]+")
GOAL_PATTERN = re.compile("\(instance (\d+)\) to \w+\((\w+)\) such that (\w+) == ([0-9.]+) with window (\d+)$")


@dataclass(init=True, repr=True, frozen=True)
class Goal:
    obj: str
    constraint: str
    target: float
    window: float


@dataclass(init=True, repr=True)
class IterationLog:
    data: Dict[str, float]

    def __contains__(self, key) -> bool:
        return key in self.data

    def __getitem__(self, key) -> float:
        return self.data[key]

@dataclass(init=True, repr=True, frozen=True)
class XupLog:
    instance: float
    measured: float
    workload: float
    derivative: float
    xup: float
    sched_xup: Optional[float]


@dataclass(init=True, repr=True, frozen=True)
class WaslLog:
    instance: float
    workload: float
    pole: float
    derivative: float
    inaccuracy: float
    e: float
    eo: float
    eoo: float

class MetricIterator:
    def __init__(self, name: str, iteration_logs: List[IterationLog]) -> None:
        self.name = name
        self.log_iterator = iter(iteration_logs)

    def __iter__(self):
        return self

    def __next__(self) -> float:
        return next(self.log_iterator)[self.name]


@dataclass(init=True)
class InstanceLogs:
    goal: Goal
    iteration_logs: List[IterationLog] = field(default_factory=list)
    xup_logs: List[XupLog] = field(default_factory=list)
    wasl_logs: List[WaslLog] = field(default_factory=list)

    def value_iterator(self, name: str) -> MetricIterator:
        if not self.iteration_logs or name not in self.iteration_logs[0]:
            return MetricIterator(name, [])
        return MetricIterator(name, self.iteration_logs)


class ExecutionLog:
    def __init__(self, fname: Path, fill_gaps=False) -> None:
        instance_logs = {}

        with fname.open() as f:
            lines = f.readlines()

        for line in lines:
            goal_match = GOAL_PATTERN.findall(line)
            if goal_match:
                inst_id, obj, constraint, target, window = goal_match[0]
                inst_id = float(inst_id)
                goal = Goal(obj, constraint, float(target), float(window))
                instance_logs[inst_id] = InstanceLogs(goal)
                continue

            matches = ITERATION_LOG_PATTERN.findall(line)
            data = {}
            for entry in matches:
                key, value = entry.strip().split(":")
                data[key] = float(value)

            if "xup" not in data and "powerConsumption" not in data:
                continue

            if "tag" not in data:
                log_entry = IterationLog(data)
                inst_id = log_entry.data["instance"]
                instance_logs[inst_id].iteration_logs.append(log_entry)
            else:
                data["instance"] = data["tag"]
                del data["tag"]

                if "pole" in data:
                    log_entry = WaslLog(**data)
                    instance_logs[log_entry.instance].wasl_logs.append(log_entry)
                else:
                    log_entry = XupLog(**data)
                    instance_logs[log_entry.instance].xup_logs.append(log_entry)

        if not fill_gaps:
            self.instance_logs = [instance_logs[i] for i in range(len(instance_logs))]
        else:
            self.instance_logs = [instance_logs[i] for i in sorted(instance_logs.keys())]

    def avg_power(self) -> float:
        values = list(self.instance_logs[-1].value_iterator("powerConsumption"))
        return np.mean(values)

    def latency_percentile(self, ptile, calc_points) -> List[float]:
        percentiles = []
        for instance, calc_point in zip(self.instance_logs, calc_points):
            values = list(instance.value_iterator("appLatency"))
            if not values:
                continue
            percentiles.append(np.percentile(values[calc_point:], ptile) / 1_000_000)
        return percentiles

    def latency_mean(self, instance, window):
        return running_mean(self.instance_logs[instance].value_iterator("appLatency"), window)

    def mapes(self, calc_points) -> List[float]:
        instance_mapes = {}

        for idx, (entry, calc_point) in enumerate(zip(self.instance_logs, calc_points)):
            constraint_values = entry.value_iterator(entry.goal.constraint)
            meaned_constraint_values = running_mean(constraint_values, entry.goal.window)

            subset = meaned_constraint_values[calc_point:]
            instance_mapes[idx] = mape(np.zeros(len(subset)) + entry.goal.target, subset)

        return instance_mapes

    def workloads(self) -> List[float]:
        workloads = []
        for entry in self.instance_logs:
            instance_workloads = [1 / l.workload for l in entry.xup_logs][-15:]
            workloads.append(np.mean(instance_workloads))
        return workloads
