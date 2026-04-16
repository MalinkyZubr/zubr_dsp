from typing import Any

import numpy as np
import scipy as scp
import matplotlib.pyplot as plt
from numpy import ndarray, dtype

CARRIER_AMPLITUDE = 10
CARRIER_FREQUENCY = 200
MODULATION_INDEX = 0.99
SAMPLE_PERIOD = 1 / (2 * CARRIER_FREQUENCY)
PHASE_INCREMENT = 2 * np.pi * CARRIER_FREQUENCY * SAMPLE_PERIOD

def am_mod(input_signal: list[float]) -> list[float]:
    phase_accumulator = 0
    output = []
    for input_sample in input_signal:
        output_sample = CARRIER_AMPLITUDE * (1 + MODULATION_INDEX * input_sample) * np.cos(phase_accumulator)
        output.append(output_sample)
        phase_accumulator = (PHASE_INCREMENT + phase_accumulator) % (2 * np.pi)

    return output


def am_demod(am_signal: list[float]) -> ndarray[tuple[int, ...], dtype[Any]]:
    input_signal_ft = np.fft.fft(am_signal)
    for bin_ in range(1, len(input_signal_ft) // 2):
        input_signal_ft[bin_] *= 2
    for bin_ in range(len(input_signal_ft) // 2 + 1, len(input_signal_ft)):
        input_signal_ft[bin_] *= 0
        
    analytic_td = np.fft.ifft(input_signal_ft)
    return (np.abs(analytic_td) / CARRIER_AMPLITUDE - 1) / MODULATION_INDEX


if __name__ == "__main__":
    time = np.arange(0, 65536) * SAMPLE_PERIOD
    signal = np.cos(2 * np.pi * 20 * time) # 1 hz sine wave
    am_signal = np.array(am_mod(signal))
    
    am_demod_signal = am_demod(am_signal)

    plt.plot(time, signal)
    plt.show()

    plt.plot(time, am_signal)
    plt.show()
    
    plt.plot(time, am_demod_signal)
    plt.plot(time, signal)
    plt.legend(["AM demod", "Original"])
    plt.show()
