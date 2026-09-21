#!/usr/bin/env python3
"""Visualize a windmeter log (real SD-card CSV or `cargo run --bin simulate`'s
output -- same format, `LogRecord::CSV_HEADER` in src/logger.rs).

Usage:
    python3 scripts/visualize_log.py path/to/log.csv [-o output.html] [--no-show]

Needs pandas + plotly (already in this project's .venv: run via
`.venv/bin/python scripts/visualize_log.py ...`; otherwise
`pip install pandas plotly`).

Produces an interactive HTML plot (speed, direction, and GPS
speed-over-ground each as their own panel -- different units never share
an axis) and opens it in a browser unless --no-show is given.
"""

import argparse
import sys
from pathlib import Path

import pandas as pd
import plotly.graph_objects as go
from plotly.subplots import make_subplots

# Categorical colors from the project's dataviz palette (validated
# colorblind-safe adjacent pair): slot 1 (blue) and slot 2 (orange).
APPARENT_COLOR = "#2a78d6"
TRUE_COLOR = "#eb6834"
GPS_COLOR = "#1baf7a"  # slot 3 (aqua)


def load_log(path: Path) -> pd.DataFrame:
    df = pd.read_csv(path)
    df["t_s"] = (df["timestamp_ms"] - df["timestamp_ms"].iloc[0]) / 1000.0
    return df


def plot(df: pd.DataFrame, title: str) -> go.Figure:
    n_invalid_apparent = (~df["apparent_valid"]).sum()
    n_invalid_true = (~df["true_valid"]).sum()
    n_invalid_gps = (~df["gps_valid"]).sum()

    fig = make_subplots(
        rows=3,
        cols=1,
        shared_xaxes=True,
        vertical_spacing=0.08,
        subplot_titles=("Wind speed (m/s)", "Wind direction (deg)", "GPS speed over ground (m/s)"),
    )

    def add(row, y_col, valid_col, color, name):
        valid = df[df[valid_col]]
        fig.add_trace(
            go.Scatter(
                x=valid["t_s"],
                y=valid[y_col],
                mode="lines+markers",
                name=name,
                line={"color": color, "width": 2},
                marker={"size": 5},
                legendgroup=name,
                showlegend=(row == 1),
            ),
            row=row,
            col=1,
        )

    add(1, "apparent_speed_mps", "apparent_valid", APPARENT_COLOR, "Apparent")
    add(1, "true_speed_mps", "true_valid", TRUE_COLOR, "True")
    add(2, "apparent_dir_deg", "apparent_valid", APPARENT_COLOR, "Apparent")
    add(2, "true_dir_deg", "true_valid", TRUE_COLOR, "True")
    add(3, "sog_mps", "gps_valid", GPS_COLOR, "GPS SOG")

    fig.update_yaxes(title_text="m/s", row=1, col=1)
    fig.update_yaxes(title_text="deg", range=[0, 360], row=2, col=1)
    fig.update_yaxes(title_text="m/s", row=3, col=1)
    fig.update_xaxes(title_text="seconds since first record", row=3, col=1)

    subtitle = f"{len(df)} records"
    if n_invalid_apparent or n_invalid_true or n_invalid_gps:
        subtitle += (
            f" ({n_invalid_apparent} invalid apparent, {n_invalid_true} invalid true, "
            f"{n_invalid_gps} invalid GPS -- dropped from their lines above)"
        )
    fig.update_layout(
        title=f"{title}<br><sup>{subtitle}</sup>",
        template="plotly_white",
        hovermode="x unified",
        legend={"orientation": "h", "y": 1.08},
    )
    return fig


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("log_csv", type=Path, help="path to a windmeter log CSV")
    parser.add_argument("-o", "--output", type=Path, default=None, help="HTML output path (default: <log>.html)")
    parser.add_argument("--no-show", action="store_true", help="write the HTML file but don't open a browser")
    args = parser.parse_args()

    if not args.log_csv.exists():
        print(f"error: {args.log_csv} not found", file=sys.stderr)
        return 1

    df = load_log(args.log_csv)
    fig = plot(df, title=args.log_csv.name)

    out_path = args.output or args.log_csv.with_suffix(".html")
    fig.write_html(out_path)
    print(f"Wrote {out_path}")

    if not args.no_show:
        fig.show()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
