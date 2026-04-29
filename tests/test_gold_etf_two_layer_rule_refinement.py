import importlib.util
from pathlib import Path
import sys


SCRIPT_PATH = Path(r"E:\SM\scripts\research\gold_etf_two_layer_rule_refinement.py")


def load_module():
    spec = importlib.util.spec_from_file_location("gold_etf_two_layer_rule_refinement", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_build_small_range_space_freezes_first_layer_and_disables_third_layer():
    module = load_module()

    space = module.build_small_range_space()

    assert space["first_entry_weight"] == [0.50]
    assert space["allow_third_entry"] == [False]
    assert space["third_entry_weight"] == [0.0]


def test_build_small_range_space_limits_second_layer_and_exit_search():
    module = load_module()

    space = module.build_small_range_space()

    assert space["second_entry_trigger_drawdown"] == [-0.03, -0.04, -0.05]
    assert space["second_entry_weight"] == [0.25, 0.30, 0.35, 0.40]
    assert space["second_entry_requires_parent_signal"] == [False, True]
    assert space["max_hold_days"] == [15, 20]
    assert space["rebound_check_day"] == [5, 7]


def test_build_refinement_comparison_anchors_on_formal_two_layer_rule():
    module = load_module()
    ranked = __import__("pandas").DataFrame(
        [
            {
                "config_name": "f0.50_s1_-0.04_1_0.35_t0_0.00_0_0.00_h20_r5",
                "total_return": 0.91,
                "max_drawdown": -0.14,
                "return_drawdown_ratio": 6.4,
            },
            {
                "config_name": "f0.50_s1_-0.05_1_0.40_t0_0.00_0_0.00_h20_r5",
                "total_return": 1.12,
                "max_drawdown": -0.14,
                "return_drawdown_ratio": 7.9,
            },
        ]
    )

    comparison = module.build_refinement_comparison(ranked)

    assert set(comparison["comparison_role"]) == {"formal_two_layer_baseline", "optimized_best"}
    assert "f0.50_s1_-0.04_1_0.35_t0_0.00_0_0.00_h20_r5" in set(comparison["config_name"])
