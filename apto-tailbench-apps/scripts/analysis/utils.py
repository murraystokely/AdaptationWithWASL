import numpy as np
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
