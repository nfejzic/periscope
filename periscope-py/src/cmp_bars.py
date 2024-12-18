import argparse
import functools

from matplotlib import figure
import matplotlib.pyplot as plt
import numpy as np

from periscope_result import BenchResult


def cmp_median(medians: list[float], labels: list[str]):
    def compare(i, j):
        med_cmp = medians[i] - medians[j]
        if med_cmp == 0:
            return labels[i] < labels[j]
        else:
            return med_cmp

    return compare


def cmp_median_multi():
    def compare(
        i: tuple[str, dict[str, BenchResult]], j: tuple[str, dict[str, BenchResult]]
    ):
        length = len(i)
        i_sum = sum([b.hyperfine_results()[0].median for b in i[1].values()])
        j_sum = sum([b.hyperfine_results()[0].median for b in j[1].values()])
        med_i_avg = i_sum / length
        med_j_avg = j_sum / length

        med_cmp = med_i_avg - med_j_avg
        return int(med_cmp)

    return compare


def cmp_legend():
    def compare(a: str, b: str):
        all_numbers_a = list(
            filter(lambda x: len(x) > 0 and x[0].isdigit(), a.split("-"))
        )
        all_numbers_b = list(
            filter(lambda x: len(x) > 0 and x[0].isdigit(), b.split("-"))
        )

        for num_a, num_b in zip(all_numbers_a, all_numbers_b):
            num_a = int(num_a.removesuffix("b"))
            num_b = int(num_b.removesuffix("b"))

            if num_a != num_b:
                return num_a - num_b

        return 0

    return compare


def plot_cmp_bars(
    args: argparse.Namespace,
    periscope_results: dict[str, list[BenchResult]],
    figure: figure.Figure,
):
    legend = sorted(
        list(periscope_results.keys()),
        key=functools.cmp_to_key(cmp_legend()),
    )

    bar_w = (1 - 0.4) / len(legend)
    group_w = bar_w * len(legend)
    offsets = np.arange(start=-(group_w / 2), stop=group_w / 2, step=bar_w)
    cmap = plt.get_cmap("rainbow")
    colors = [cmap(val / len(legend)) for val in range(len(legend))]

    ax = figure.subplots(1, 1)
    _, ax2 = plt.subplots(figsize=(20, 10), constrained_layout=True)

    handles = []

    per_file: dict[str, dict[str, BenchResult]] = {}

    max_time = 0

    for bench_config in legend:
        for bench_result in periscope_results[bench_config]:
            b_name = (
                bench_result.name.removesuffix("-rotorized.btor2")
                .removesuffix("-35")
                .removesuffix("-10")
                .removesuffix("-1")
                .removesuffix("-2")
                .removesuffix("-3")
            )
            if b_name not in per_file:
                per_file[b_name] = {}

            max_time = np.max([max_time, bench_result.hyperfine_results()[0].median])
            per_file[b_name][bench_config] = bench_result

    title = "Comparison of median times with different model configurations"

    if args.sort_by == "median":
        per_file = dict(
            sorted(per_file.items(), key=functools.cmp_to_key(cmp_median_multi()))
        )

    if args.scale == "log":
        ax2.set_yscale("log")
        title += " (Log scale)"

    plt.title(title)
    plt.ylabel("Time [s]")

    labels = list(per_file.keys())
    label_positions = np.arange(len(labels)) - (1 - 0.4) / (len(legend) * 2)
    plt.tick_params(axis="x", length=20)
    plt.xticks(list(label_positions), labels, rotation=55, ha="right")

    for idx, file in enumerate(per_file.values()):
        for bench_idx, bench_result in enumerate(file.values()):
            offs = offsets[bench_idx]
            time = bench_result.hyperfine_results()[0].median
            color = colors[bench_idx]
            ax = plt.bar(x=idx + offs, height=time, width=bar_w, color=color)
            handles.append(ax)

    plt.ylim(0.1, max_time)
    legend = list(map(lambda x: x.removesuffix(".json").replace("-", " "), legend))
    plt.legend(handles=handles, labels=legend)
