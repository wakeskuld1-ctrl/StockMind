import importlib.util
from pathlib import Path
import sys

import pandas as pd


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_ml_hold15d_experiment.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_ml_hold15d_experiment", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_build_rule_event_dataset_filters_to_fixed_parent_rule_and_label():
    module = load_module()
    frame = pd.DataFrame(
        {
            "trade_date": pd.to_datetime(["2021-01-01", "2021-01-02", "2021-01-03"]),
            "ret_5d": [-0.03, -0.01, -0.025],
            "close_vs_ma20": [-0.02, -0.02, -0.01],
            "future_ret_15d": [0.03, -0.01, 0.02],
            "feature_a": [1, 2, 3],
        }
    )

    dataset = module.build_rule_event_dataset(frame, feature_columns=["feature_a"])

    assert len(dataset) == 1
    assert dataset["label"].iloc[0] == 1
    assert dataset["feature_a"].iloc[0] == 1


def test_build_walk_forward_splits_supports_expanding_and_rolling_windows():
    module = load_module()
    dataset = pd.DataFrame({"year": [2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023]})

    expanding = module.build_walk_forward_splits(dataset, mode="expanding", min_train_year=2016, first_valid_year=2021)
    rolling = module.build_walk_forward_splits(dataset, mode="rolling_5y", min_train_year=2016, first_valid_year=2021)

    assert expanding[0]["train_years"] == [2016, 2017, 2018, 2019, 2020]
    assert expanding[1]["train_years"] == [2016, 2017, 2018, 2019, 2020, 2021]
    assert rolling[1]["train_years"] == [2017, 2018, 2019, 2020, 2021]


def test_summarize_model_strategy_compares_model_selection_to_rule_baseline():
    module = load_module()
    predictions = pd.DataFrame(
        [
            {"model_name": "rf", "selection_policy": "top_50pct", "future_ret_15d": 0.03, "selected_flag": 1, "score": 0.8},
            {"model_name": "rf", "selection_policy": "top_50pct", "future_ret_15d": -0.01, "selected_flag": 0, "score": 0.2},
            {"model_name": "rule_baseline", "selection_policy": "all", "future_ret_15d": 0.03, "selected_flag": 1, "score": 1.0},
            {"model_name": "rule_baseline", "selection_policy": "all", "future_ret_15d": -0.01, "selected_flag": 1, "score": 1.0},
        ]
    )

    summary = module.summarize_model_strategy(predictions)

    rf_row = summary[(summary["model_name"] == "rf") & (summary["selection_policy"] == "top_50pct")].iloc[0]
    base_row = summary[(summary["model_name"] == "rule_baseline")].iloc[0]
    assert rf_row["sample_count"] == 1
    assert rf_row["avg_return"] == 0.03
    assert base_row["sample_count"] == 2


def test_extract_model_winner_prefers_stable_gain_over_rule_baseline():
    module = load_module()
    summary = pd.DataFrame(
        [
            {"model_name": "rule_baseline", "selection_policy": "all", "sample_count": 100, "win_rate": 0.72, "avg_return": 0.02, "median_return": 0.018, "positive_year_ratio": 0.83},
            {"model_name": "rf", "selection_policy": "top_50pct", "sample_count": 50, "win_rate": 0.78, "avg_return": 0.024, "median_return": 0.022, "positive_year_ratio": 0.83},
            {"model_name": "xgb", "selection_policy": "top_30pct", "sample_count": 30, "win_rate": 0.81, "avg_return": 0.03, "median_return": 0.01, "positive_year_ratio": 0.50},
        ]
    )

    winner = module.extract_model_winner(summary)

    assert winner["model_name"] == "rf"
