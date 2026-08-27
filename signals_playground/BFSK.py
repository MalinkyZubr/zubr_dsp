import scipy.signal as scp_signal
import numpy as np
import matplotlib.pyplot as plt
from scipy.signal import sosfiltfilt
import cmath


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


def incoherent_demodulator(sample_stream: np.array, sample_rate: int, binary_dict: tuple[int, int]) -> np.array:
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


def coherent_demodulator(sample_stream: np.array, sample_rate: int, binary_dict: tuple[int, int]) -> np.array:
    minimum_samples_per_symbol = int(np.ceil(max(sample_rate / binary_dict[0], sample_rate / binary_dict[1])))

    nco_output_sample = 1 + 0j
    phase_difference_buffer = [0, 0]
    nco_frequency = 100
    nco_accumulator = 0

    frequencies = []
    symbols = []

    for input_sample in sample_stream:
        nco_output_conj = nco_output_sample.conjugate()
        difference_signal = input_sample * nco_output_conj
        phase_offset = cmath.phase(difference_signal) % (2 * np.pi)
        frequency = phase_offset - phase_difference_buffer[0]
        frequency_error = abs(nco_frequency - frequency)
        phase_difference_buffer = [phase_offset] + [phase_difference_buffer[0]]

        nco_frequency += 100 * frequency_error
        frequencies.append(nco_frequency)
        nco_accumulator = (nco_accumulator + (nco_frequency / sample_rate)) % 1.0
        nco_output_sample = np.exp(2 * np.pi * nco_accumulator * 1j)

        if len(frequencies) == minimum_samples_per_symbol:
            avg_value = sum(frequencies) / minimum_samples_per_symbol
            if abs(avg_value - binary_dict[0]) > abs(avg_value - binary_dict[1]):
                symbols.append(1)
            else:
                symbols.append(0)
            frequencies = []


    return np.asarray(symbols)


if __name__ == "__main__":
    SAMPLE_RATE = 100000
    SYMBOLS = (100, 200)
    sos = scp_signal.butter(N=25, Wn=250, btype='low', fs=SAMPLE_RATE, output='sos')

    data = np.random.randint(0, 2, 128)
    baseband_signal = phase_accumulator_modulator(list(data), SAMPLE_RATE, SYMBOLS)
    sig_power = sum(np.square(np.abs(baseband_signal))) / baseband_signal.size
    noise = 10 * np.random.normal(0.0, 0.5, baseband_signal.shape)
    noise_power = sum(np.square(np.abs(noise))) / noise.size
    baseband_signal += noise

    #baseband_signal = sosfiltfilt(sos, baseband_signal)

    snr = 10 * np.log10(sig_power / noise_power)

    print(f"SNR: {snr}")

    incoherent_demodulated = np.asarray(incoherent_demodulator(baseband_signal, SAMPLE_RATE, SYMBOLS))
    berror_rate = np.sum(data != incoherent_demodulated) / len(data)
    print(f"Incoherent Bit Error Rate: {berror_rate}")
    x = np.arange(0, 128)
    plt.step(x, data, label="Original Waveform")
    plt.step(x, incoherent_demodulated + 1.1, label="incoherent demodulated")

    coherent_demodulated = coherent_demodulator(baseband_signal, SAMPLE_RATE, SYMBOLS)
    berror_rate = np.sum(data != coherent_demodulated) / len(data)
    print(f"Coherent Bit Error Rate: {berror_rate}")

    x = np.arange(0, 128)
    plt.step(x, coherent_demodulated + 2.2, label="coherent demodulated")
    plt.legend()
    plt.show()