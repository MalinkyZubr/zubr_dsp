import scipy.signal as scp_signal
import numpy as np
import matplotlib.pyplot as plt
from scipy.signal import sosfiltfilt


def phase_accumulator_modulator(datastream: list[int], sample_rate: int, binary_dict: tuple[int, int]) -> np.array:
    minimum_samples_per_symbol = int(np.ceil(max(sample_rate / binary_dict[0], sample_rate / binary_dict[1])))
    output: list[np.complex64] = []

    accumulated_value: int = 0
    for datapoint in datastream:
        for sample in range(minimum_samples_per_symbol):
            output.append(np.exp(2 * np.pi * accumulated_value * 1j))
            accumulated_value = (accumulated_value + (binary_dict[datapoint] / sample_rate)) % 1.0

    return np.asarray(output)


def energy_correlator(recv_buff: np.array, sym_buff: np.array, sample_rate: float) -> float:
    sos = scp_signal.butter(N=2, Wn=50, btype='low', fs=sample_rate, output='sos')

    filtered_data = np.abs(np.sum(scp_signal.sosfiltfilt(sos, np.multiply(recv_buff, sym_buff))) ** 2)
    return filtered_data


def incoherent_demodulator(sample_stream: list[float], sample_rate: int, binary_dict: tuple[int, int]) -> np.array:
    minimum_samples_per_symbol = int(np.ceil(max(sample_rate / binary_dict[0], sample_rate / binary_dict[1])))

    in_phase_sym_0 = phase_accumulator_modulator([0], sample_rate, binary_dict)
    quad_phase_sym_0 = 1j * in_phase_sym_0

    in_phase_sym_1 = phase_accumulator_modulator([1], sample_rate, binary_dict)
    quad_phase_sym_1 = 1j * in_phase_sym_1

    data_buff = []
    sample_buff = []

    for start_index in range(0, len(sample_stream), minimum_samples_per_symbol):
        symbol_window = sample_stream[start_index:start_index + minimum_samples_per_symbol]
        sym_0_comparator = energy_correlator(symbol_window, np.conj(in_phase_sym_0), sample_rate)# + energy_correlator(symbol_window, quad_phase_sym_0, sample_rate)
        sym_1_comparator = energy_correlator(symbol_window, np.conj(in_phase_sym_1), sample_rate)# + energy_correlator(symbol_window, quad_phase_sym_1, sample_rate)

        sample_buff += [sym_0_comparator - sym_1_comparator]
        if (sym_0_comparator - sym_1_comparator) > 0:
            data_buff.append(0)
        else:
            data_buff.append(1)

    return data_buff


if __name__ == "__main__":
    SAMPLE_RATE = 1000
    SYMBOLS = (10, 20)
    sos = scp_signal.butter(N=25, Wn=30, btype='low', fs=SAMPLE_RATE, output='sos')

    data = np.random.randint(0, 2, 128)
    baseband_signal = phase_accumulator_modulator(list(data), SAMPLE_RATE, SYMBOLS)
    baseband_signal += 1 * np.random.normal(0.0, 0.5, baseband_signal.shape)

    baseband_signal = sosfiltfilt(sos, baseband_signal)
    demodulated = np.asarray(incoherent_demodulator(list(baseband_signal), SAMPLE_RATE, SYMBOLS))

    snr = scp_signal
    berror_rate = np.sum(data != demodulated) / len(data)
    print(f"Bit Error Rate: {berror_rate}")
    x = np.arange(0, 128)
    plt.step(x, data)
    plt.step(x, demodulated + 2)
    plt.show()

    plt.plot(baseband_signal)
    plt.show()
