use crate::ops::stock::security_forward_outcome::SecurityForwardOutcomeDocument;

// 2026-04-14 CST: Extracted from security_post_trade_review.rs because round 2
// needs one dedicated bottom-layer review policy module.
// Purpose: keep review-state classification and next-step rules governed in one
// reusable place instead of leaving them embedded inside the review assembler.
pub(crate) struct SecurityPostTradeReviewPolicy;

impl SecurityPostTradeReviewPolicy {
    pub(crate) fn review_status(position_state: &str) -> String {
        if position_state == "open" {
            "open_position_pending".to_string()
        } else {
            "completed".to_string()
        }
    }

    pub(crate) fn thesis_status(
        review_status: &str,
        selected_outcome: &SecurityForwardOutcomeDocument,
    ) -> String {
        if review_status == "open_position_pending" {
            "pending".to_string()
        } else {
            classify_thesis_status(selected_outcome)
        }
    }

    pub(crate) fn execution_deviation(review_status: &str, execution_quality: &str) -> String {
        if review_status == "open_position_pending" {
            "open_position_pending".to_string()
        } else {
            execution_quality.to_string()
        }
    }

    pub(crate) fn model_miss_reason(
        review_status: &str,
        selected_outcome: &SecurityForwardOutcomeDocument,
        thesis_status: &str,
        execution_deviation: &str,
    ) -> String {
        if review_status == "open_position_pending" {
            "review_pending_until_position_closed".to_string()
        } else {
            derive_model_miss_reason(selected_outcome, thesis_status, execution_deviation)
        }
    }

    pub(crate) fn next_adjustment_hint(
        review_status: &str,
        thesis_status: &str,
        position_risk_grade: &str,
        selected_outcome: &SecurityForwardOutcomeDocument,
        execution_deviation: &str,
    ) -> String {
        if review_status == "open_position_pending" {
            "position still open; continue tracking exit conditions, tranche discipline, and account budget before final review".to_string()
        } else {
            derive_next_adjustment_hint(
                thesis_status,
                position_risk_grade,
                selected_outcome,
                execution_deviation,
            )
        }
    }

    pub(crate) fn tranche_discipline(account_plan_alignment: &str) -> String {
        classify_tranche_discipline(account_plan_alignment)
    }

    pub(crate) fn budget_drift_reason(account_plan_alignment: &str) -> String {
        derive_budget_drift_reason(account_plan_alignment)
    }

    pub(crate) fn next_account_adjustment_hint(account_plan_alignment: &str) -> String {
        derive_next_account_adjustment_hint(account_plan_alignment)
    }
}

fn classify_thesis_status(selected_outcome: &SecurityForwardOutcomeDocument) -> String {
    if selected_outcome.hit_stop_first {
        "broken".to_string()
    } else if selected_outcome.forward_return > 0.0 && selected_outcome.max_drawdown <= 0.08 {
        "validated".to_string()
    } else if selected_outcome.forward_return > 0.0 {
        "mixed".to_string()
    } else {
        "broken".to_string()
    }
}

fn derive_model_miss_reason(
    selected_outcome: &SecurityForwardOutcomeDocument,
    thesis_status: &str,
    execution_deviation: &str,
) -> String {
    if execution_deviation == "adverse" && thesis_status == "validated" {
        return "execution_slippage_overrode_valid_thesis".to_string();
    }
    if thesis_status == "validated" {
        return "none".to_string();
    }
    if selected_outcome.hit_stop_first {
        return "stop_loss_triggered_before_thesis_played_out".to_string();
    }
    if selected_outcome.forward_return <= 0.0 {
        return "negative_forward_return_within_review_window".to_string();
    }
    "reward_realized_but_path_quality_weakened".to_string()
}

fn derive_next_adjustment_hint(
    thesis_status: &str,
    position_risk_grade: &str,
    selected_outcome: &SecurityForwardOutcomeDocument,
    execution_deviation: &str,
) -> String {
    if execution_deviation == "adverse" {
        return "tighten entry slippage, size drift, and exit discipline before reusing the thesis"
            .to_string();
    }
    match thesis_status {
        "validated" if selected_outcome.max_runup >= 0.10 => {
            "keep the thesis active and reuse the current sizing framework for similar setups"
                .to_string()
        }
        "validated" => {
            "keep the thesis active but wait for stronger evidence of repeatable edge".to_string()
        }
        "mixed" if position_risk_grade == "high" => {
            "reduce initial size for similar high-risk setups and tighten drawdown path rules"
                .to_string()
        }
        "mixed" => {
            "keep the directional view but lower size or delay add-on confirmation next time"
                .to_string()
        }
        _ => "downgrade similar setups until stronger confirmation appears".to_string(),
    }
}

fn classify_tranche_discipline(account_plan_alignment: &str) -> String {
    match account_plan_alignment {
        "aligned" => "disciplined".to_string(),
        "under_budget" => "underfilled".to_string(),
        "over_budget" => "overfilled".to_string(),
        _ => "offside".to_string(),
    }
}

fn derive_budget_drift_reason(account_plan_alignment: &str) -> String {
    match account_plan_alignment {
        "aligned" => "none".to_string(),
        "under_budget" => "planned_tranche_not_fully_executed".to_string(),
        "over_budget" => "executed_tranche_exceeded_account_budget".to_string(),
        _ => "execution_direction_conflicted_with_account_plan".to_string(),
    }
}

fn derive_next_account_adjustment_hint(account_plan_alignment: &str) -> String {
    match account_plan_alignment {
        // 2026-04-17 CST: Reason=the formal post-trade contract is already locked by CLI
        // tests with Chinese governance hints. Purpose=restore the stable operator-facing
        // wording after an English drift regressed the review output contract.
        "aligned" => "继续沿用当前账户预算与分层执行纪律。".to_string(),
        "under_budget" => "先确认计划层数为何未执行完整，再决定是否补回剩余仓位。".to_string(),
        "over_budget" => "下次同类机会先回到计划层数，未重新通过账户预算复核前不要继续追加强度。"
            .to_string(),
        _ => "先暂停原账户动作，重新核对方向、预算与分层模板后再恢复执行。".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::SecurityPostTradeReviewPolicy;

    #[test]
    fn review_policy_marks_open_position_as_pending() {
        assert_eq!(
            SecurityPostTradeReviewPolicy::review_status("open"),
            "open_position_pending"
        );
        assert_eq!(
            SecurityPostTradeReviewPolicy::execution_deviation("open_position_pending", "aligned"),
            "open_position_pending"
        );
    }

    #[test]
    fn review_policy_maps_budget_alignment_to_tranche_discipline() {
        assert_eq!(
            SecurityPostTradeReviewPolicy::tranche_discipline("over_budget"),
            "overfilled"
        );
        assert_eq!(
            SecurityPostTradeReviewPolicy::budget_drift_reason("under_budget"),
            "planned_tranche_not_fully_executed"
        );
    }
}
