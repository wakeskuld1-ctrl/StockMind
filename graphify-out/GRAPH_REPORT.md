# Graph Report - E:\SM  (2026-04-20)

## Corpus Check
- Large corpus: 210 files · ~233,192 words. Semantic extraction will be expensive (many Claude tokens). Consider running on a subfolder, or use --no-semantic to run AST-only.

## Summary
- 2756 nodes · 4922 edges · 102 communities detected
- Extraction: 59% EXTRACTED · 41% INFERRED · 0% AMBIGUOUS · INFERRED: 2022 edges (avg confidence: 0.5)
- Token cost: 0 input · 0 output

## God Nodes (most connected - your core abstractions)
1. `build_consultation_result()` - 24 edges
2. `build_security_portfolio_replacement_plan()` - 16 edges
3. `build_indicator_snapshot()` - 16 edges
4. `security_analysis_fullstack()` - 15 edges
5. `http_get_text()` - 15 edges
6. `SignalOutcomeStore` - 15 edges
7. `StockHistoryStore` - 15 edges
8. `create_stock_history_csv()` - 15 edges
9. `spawn_http_route_server()` - 15 edges
10. `import_history_csv()` - 15 edges

## Surprising Connections (you probably didn't know these)
- None detected - all connections are within the same source files.

## Communities

### Community 0 - "Build Decision"
Cohesion: 0.02
Nodes (135): canonicalize_json_bytes(), canonicalize_json_value(), SecurityApprovalBriefSignatureEnvelope, sha256_hex(), sign_security_approval_brief_document(), to_hex(), verify_security_approval_brief_document(), build_review_notes() (+127 more)

### Community 1 - "Build Disclosure"
Cohesion: 0.04
Nodes (124): append_query_params(), build_available_cross_border_leg(), build_available_disclosure_context(), build_available_fundamental_context(), build_builtin_etf_context(), build_cross_border_context(), build_cross_border_leg_analysis(), build_cross_border_premium_assessment() (+116 more)

### Community 2 - "Build With"
Cohesion: 0.03
Nodes (124): adx_snapshot(), atr_last(), bollinger_bandwidth_classifies_contracting_at_exact_lower_threshold(), bollinger_bandwidth_classifies_expanding_at_exact_upper_threshold(), bollinger_bandwidth_stays_normal_just_above_contracting_threshold(), bollinger_bandwidth_stays_normal_just_below_expanding_threshold(), bollinger_classifies_lower_band_rebound_candidate_at_exact_lower_band(), bollinger_classifies_upper_band_breakout_risk_at_exact_upper_band() (+116 more)

### Community 3 - "Dispatch Post"
Cohesion: 0.02
Nodes (40): build_promotion_document(), load_model_registry(), load_shadow_evaluation(), persist_json(), resolve_runtime_root(), security_model_promotion(), SecurityModelPromotionDocument, SecurityModelPromotionError (+32 more)

### Community 4 - "Build Default"
Cohesion: 0.03
Nodes (99): compare_desc(), compare_scores(), default_ranking_policy(), DirectionFirstRankingScore, load_model_registry(), metric_f64(), metric_string(), persist_json() (+91 more)

### Community 5 - "Proxy Parse"
Cohesion: 0.03
Nodes (80): build_header_index_map(), build_import_result(), field_value(), import_stock_price_history(), ImportDateRange, ImportStockPriceHistoryError, ImportStockPriceHistoryRequest, ImportStockPriceHistoryResult (+72 more)

### Community 6 - "Build Prediction"
Cohesion: 0.04
Nodes (84): action_bias_from_actionability(), action_bias_label(), build_briefing_digest(), build_evidence_checks(), build_execution_digest(), build_historical_digest(), build_odds_digest(), build_position_digest() (+76 more)

### Community 7 - "Default Resonance"
Cohesion: 0.03
Nodes (71): align_tail(), AppendResonanceEventTagsRequest, AppendResonanceEventTagsResult, AppendResonanceFactorSeriesRequest, AppendResonanceFactorSeriesResult, average(), bootstrap_resonance_template_factors(), BootstrapResonanceTemplateFactorsRequest (+63 more)

### Community 8 - "Execution Adjustment"
Cohesion: 0.04
Nodes (39): PositionAdjustmentEventType, PositionPlanAlignment, SecurityPositionPlanRecordRequest, SecurityPositionPlanRecordResult, SecurityRecordPositionAdjustmentRequest, SecurityRecordPositionAdjustmentResult, ToolRequest, ToolResponse (+31 more)

### Community 9 - "Position Normalize"
Cohesion: 0.04
Nodes (69): build_approved_candidate_entries(), build_live_positions(), build_security_account_objective_contract(), normalize_created_at(), normalize_lowercase(), normalize_optional_text(), normalize_symbol(), normalize_text() (+61 more)

### Community 10 - "Build Default"
Cohesion: 0.05
Nodes (46): build_chair_reasoning(), build_committee_reason(), build_execution_constraints(), build_quant_reason(), build_security_chair_resolution(), ChairArbitrationOutcome, dedupe_strings(), derive_arbitration_outcome() (+38 more)

### Community 11 - "Fundamental Resolve"
Cohesion: 0.05
Nodes (47): build_context_from_metrics(), build_fundamental_narrative(), build_record_ref(), classify_fundamental_signal(), collect_covered_report_periods(), collect_unique_symbol_count(), load_historical_fundamental_context(), persist_json() (+39 more)

### Community 12 - "Signal Default"
Cohesion: 0.05
Nodes (39): AnalogStudySummaryPayload, average(), average_option(), backfill_security_signal_outcomes(), BackfillSecuritySignalOutcomesRequest, BackfillSecuritySignalOutcomesResult, build_forward_return_row(), build_indicator_digest() (+31 more)

### Community 13 - "Build Entry"
Cohesion: 0.06
Nodes (54): apply_security_entry_layer_to_position_plan(), apply_security_sizing_assessment_to_position_plan(), apply_security_sizing_layer_to_position_plan(), build_add_notes(), build_add_trigger_condition(), build_cancel_conditions(), build_entry_mode_from_entry_grade(), build_entry_trigger_condition() (+46 more)

### Community 14 - "Execution Fixture()"
Cohesion: 0.05
Nodes (41): AccountExecutionSummary, AccountPlanBinding, classify_account_budget_alignment(), SecurityExecutionAccountBindingResolver, tranche_units_for_account_plan(), account_binding_resolver_reads_matching_allocation(), adapt_execution_record_request(), analysis_date_guard_fixture() (+33 more)

### Community 15 - "Fixture Build"
Cohesion: 0.05
Nodes (42): BiasDirection, build_basic_request(), build_contextual_conclusion(), map_bias_direction(), resolve_market_symbol(), resolve_sector_symbol(), security_analysis_contextual(), SecurityAnalysisContextualError (+34 more)

### Community 16 - "Default Pct()"
Cohesion: 0.05
Nodes (35): build_security_execution_journal(), normalize_created_at(), normalize_lines(), parse_date(), rounded_pct(), security_execution_journal(), SecurityExecutionJournalDocument, SecurityExecutionJournalError (+27 more)

### Community 17 - "Committee Build"
Cohesion: 0.08
Nodes (53): aggregate_committee_votes(), build_chair_opinion(), build_committee_votes(), build_event_opinion(), build_execution_instance_id(), build_execution_opinion(), build_fundamental_opinion(), build_meeting_digest() (+45 more)

### Community 18 - "Build Committee"
Cohesion: 0.06
Nodes (45): assemble_security_decision_briefing(), base_position_limits_by_odds(), build_committee_payload(), build_committee_recommendation_entry(), build_committee_recommendations(), build_committee_risk_breakdown(), build_evidence_version(), build_execution_plan() (+37 more)

### Community 19 - "Open Position"
Cohesion: 0.07
Nodes (36): derive_requested_as_of_date(), SecurityAccountOpenPositionSnapshotAssembler, SecurityAccountOpenPositionSnapshotAssembler<'a>, snapshot_assembler_maps_runtime_records_into_contract(), temp_db_path(), build_review_rows(), create_stock_history_csv(), execution_record_fixture() (+28 more)

### Community 20 - "Derive Build"
Cohesion: 0.06
Nodes (38): build_evidence_bundle(), build_evidence_bundle_feature_seed(), build_evidence_hash(), build_evidence_quality(), build_fallback_disclosure_signal_summary(), build_governed_disclosure_signal_summary(), build_governed_shareholder_return_status(), classify_asset_class() (+30 more)

### Community 21 - "Feature Snapshot"
Cohesion: 0.09
Nodes (39): build_disclosure_headline(), build_disclosure_keyword_summary(), build_disclosure_risk_flags(), build_record_ref(), collect_covered_published_dates(), collect_unique_symbol_count(), contains_any(), load_historical_disclosure_context() (+31 more)

### Community 22 - "History .Load"
Cohesion: 0.07
Nodes (32): build_open_position_corporate_action_summary(), CorporateActionEffect, evaluate_corporate_action_effect(), fixture_record(), holding_summary_applies_cash_dividend_to_breakeven_and_total_return(), holding_summary_applies_split_bonus_and_dividend_to_live_position_math(), normalized_split_factor(), OpenPositionCorporateActionSummary (+24 more)

### Community 23 - "Build Default"
Cohesion: 0.07
Nodes (29): average(), build_current_snapshot_and_verdict(), build_fx_assessment(), build_premium_assessment(), build_replay_summary_and_cases(), build_triggered_case(), build_underlying_market_assessment(), compute_forward_return() (+21 more)

### Community 24 - "Rows() Provider"
Cohesion: 0.11
Nodes (37): append_query_param(), build_fred_series_url(), build_sina_url(), build_sync_result(), build_tencent_url(), date_in_window(), fetch_fred_rows(), fetch_provider_rows() (+29 more)

### Community 25 - "Pct() Default"
Cohesion: 0.07
Nodes (32): build_adjustment_simulation_data(), build_capital_rebalance_simulation(), build_monitoring_account_aggregation(), build_portfolio_position_plan(), build_ranked_candidates(), candidate_priority_score(), confidence_score(), derived_max_tranche_count() (+24 more)

### Community 26 - "Build Decision"
Cohesion: 0.11
Nodes (28): apply_committee_vote_to_decision_card(), apply_risk_veto_to_decision_card(), apply_training_guardrail_to_decision_card(), base_committee_score(), build_bear_case(), build_bull_case(), build_member_opinion(), build_member_opinions() (+20 more)

### Community 27 - "Shadow Derive"
Cohesion: 0.1
Nodes (27): build_shadow_evaluation_document(), build_window_metric_snapshot(), collect_window_metric_snapshots(), derive_oot_stability_status(), derive_promotion_blockers(), derive_promotion_evidence_notes(), derive_proxy_coverage_status(), derive_recommended_model_grade() (+19 more)

### Community 28 - "Position Build"
Cohesion: 0.1
Nodes (27): build_rebased_position_contracts(), build_security_account_rebase_snapshot(), build_security_capital_event(), build_security_capital_rebalance_evidence_package(), normalize_created_at(), normalize_optional_text(), round_amount(), round_pct() (+19 more)

### Community 29 - "Build Replacement"
Cohesion: 0.13
Nodes (25): build_conflict_resolution_summary(), build_current_weights(), build_entry_actions(), build_exit_actions(), build_replacement_pairs(), build_security_portfolio_replacement_plan(), build_target_weights(), build_trim_actions() (+17 more)

### Community 30 - "Submit Approval"
Cohesion: 0.28
Nodes (25): build_confirmed_breakout_rows(), collect_ready_scorecard_artifact_candidates(), create_ready_submit_approval_scorecard_artifact(), create_scorecard_artifact_fixture(), create_scorecard_registry_fixture(), create_shadow_evaluation_fixture(), create_stock_history_csv(), ensure_other_bucket() (+17 more)

### Community 31 - "Package Decision"
Cohesion: 0.25
Nodes (20): build_confirmed_breakout_rows(), create_json_fixture(), create_minimal_verifiable_package_fixture(), create_stock_history_csv(), import_history_csv(), resolve_post_meeting_artifact_path(), rewrite_package_json(), rewrite_post_meeting_artifact() (+12 more)

### Community 32 - "Factor .Upsert"
Cohesion: 0.15
Nodes (8): arithmetic_mean(), ResonanceEventTag, ResonanceFactorDefinition, ResonanceFactorPoint, SecurityResonanceSnapshotRow, SecurityResonanceStore, SecurityResonanceStoreError, standard_deviation()

### Community 33 - "Runtime Path()"
Cohesion: 0.18
Nodes (7): execution_store_db_path_falls_back_to_runtime_root(), execution_store_db_path_prefers_direct_env_override(), execution_store_runtime_dir_uses_execution_db_parent_when_overridden(), FormalSecurityRuntimeRegistry, governed_runtime_store_db_paths_fall_back_to_shared_runtime_root(), governed_runtime_store_db_paths_prefer_direct_env_overrides(), RuntimeEnvGuard

### Community 34 - "Chair Resolution"
Cohesion: 0.39
Nodes (17): build_classification_head_artifact_json(), build_confirmed_breakout_rows(), build_etf_direction_artifact_json(), build_regression_head_artifact_json(), create_stock_history_csv(), import_history_csv(), security_chair_resolution_accepts_treasury_etf_subscope_artifact(), security_chair_resolution_does_not_require_stock_only_information_for_gold_etf_when_proxy_history_is_complete() (+9 more)

### Community 35 - "Derive Reason()"
Cohesion: 0.15
Nodes (7): classify_thesis_status(), classify_tranche_discipline(), derive_budget_drift_reason(), derive_model_miss_reason(), derive_next_account_adjustment_hint(), derive_next_adjustment_hint(), SecurityPostTradeReviewPolicy

### Community 36 - "Position Document()"
Cohesion: 0.15
Nodes (6): per_position_evaluation_add_document(), position_contract_accumulate_document(), position_contract_trim_document(), security_monitoring_evidence_package_rejects_evaluation_account_mismatch(), security_monitoring_evidence_package_rejects_position_contract_account_mismatch(), security_monitoring_evidence_package_surfaces_high_risk_budget_pressure_warning()

### Community 37 - "Portfolio Replacement"
Cohesion: 0.18
Nodes (9): build_p10_documents(), security_portfolio_replacement_plan_builds_unified_plan_from_p10_outputs(), security_portfolio_replacement_plan_carries_rebase_context_into_capital_migration_plan(), security_portfolio_replacement_plan_emits_conflict_resolution_summary(), security_portfolio_replacement_plan_rejects_candidate_boundary_drift(), security_portfolio_replacement_plan_rejects_cross_account_drift(), security_portfolio_replacement_plan_rejects_infeasible_allocation(), security_portfolio_replacement_plan_rejects_weight_non_conservation() (+1 more)

### Community 38 - "Package Revision"
Cohesion: 0.3
Nodes (14): build_chair_revision_fixture_package(), build_confirmed_breakout_rows(), create_json_fixture(), create_post_meeting_chair_fixture(), create_stock_history_csv(), import_history_csv(), run_chair_revision(), security_decision_package_revision_builds_v2_package_after_approval_update() (+6 more)

### Community 39 - "History Price"
Cohesion: 0.25
Nodes (13): create_stock_history_csv(), import_stock_price_history_defaults_adj_close_to_close_when_missing(), import_stock_price_history_imports_csv_into_sqlite(), import_stock_price_history_rejects_csv_missing_required_columns(), import_stock_price_history_replaces_existing_symbol_trade_date_rows(), spawn_http_route_server(), spawn_http_server(), stock_history_db_path() (+5 more)

### Community 40 - "Open Record"
Cohesion: 0.27
Nodes (8): apply_open_position_holding_economics(), apply_open_position_holding_economics_keeps_closed_record_empty(), apply_open_position_holding_economics_populates_live_fields_for_open_record(), derive_requested_as_of_date(), open_record_fixture(), SecurityExecutionRecordAssembler, SecurityExecutionRecordAssembler<'a>, temp_db_path()

### Community 41 - "Account Objective"
Cohesion: 0.19
Nodes (9): approved_candidate_document(), monitoring_evidence_package_document(), position_contract_accumulate_document(), position_contract_trim_document(), security_account_objective_contract_rejects_cross_account_drift(), security_account_objective_contract_rejects_duplicate_symbol_drift(), security_account_objective_contract_rejects_missing_capital_base(), security_account_objective_contract_rejects_mixed_account_candidate_input() (+1 more)

### Community 42 - "Package Decision"
Cohesion: 0.28
Nodes (13): attach_post_meeting_artifact(), build_confirmed_breakout_rows(), create_fixture_dir(), create_json_fixture(), create_stock_history_csv(), import_history_csv(), prepare_submit_package(), refresh_present_artifact_manifest_hashes() (+5 more)

### Community 43 - "Execution Record"
Cohesion: 0.3
Nodes (13): approx_equal(), build_review_rows(), create_stock_history_csv(), execution_request(), import_history_csv(), portfolio_position_plan_document_fixture(), prepare_security_environment(), security_envs() (+5 more)

### Community 44 - "Training Scorecard"
Cohesion: 0.37
Nodes (13): build_nikkei_decade_rows(), build_nikkei_mixed_regime_rows(), build_trend_rows(), build_trend_rows_keeps_low_series_variable_in_downtrend_fixture(), create_training_fixture_dir(), import_history_csv(), security_scorecard_training_generates_artifact_and_registers_refit_outputs(), security_scorecard_training_generates_nikkei_index_artifact() (+5 more)

### Community 45 - "Symbol Effective"
Cohesion: 0.25
Nodes (13): effective_routing_replaces_blended_sector_proxy_with_symbol_taxonomy(), EffectiveSecurityRouting, is_blended_profile(), load_symbol_taxonomy(), normalize_symbol(), normalized_non_empty(), normalized_owned(), resolve_effective_security_routing() (+5 more)

### Community 46 - "Analysis Fullstack"
Cohesion: 0.38
Nodes (12): build_choppy_history_rows(), build_confirmed_breakdown_rows(), build_confirmed_breakout_rows(), build_fred_csv_ending_at(), build_range_bound_rows(), create_stock_history_csv(), import_history_csv(), security_analysis_fullstack_aggregates_technical_fundamental_and_disclosures() (+4 more)

### Community 47 - "Capital Rebase"
Cohesion: 0.16
Nodes (4): monitoring_evidence_package_document(), position_contract_accumulate_document(), security_capital_rebase_rejects_monitoring_package_account_mismatch(), security_capital_rebase_rejects_position_contract_account_mismatch()

### Community 48 - "Master Scorecard"
Cohesion: 0.42
Nodes (10): build_classification_head_artifact_json(), build_linear_growth_rows(), build_regression_head_artifact_json(), create_stock_history_csv(), import_history_csv(), security_master_scorecard_attaches_path_event_context_when_available(), security_master_scorecard_preserves_partial_multi_head_summary_when_three_heads_are_available(), security_master_scorecard_returns_formal_multi_horizon_profitability_summary() (+2 more)

### Community 49 - "Review Post"
Cohesion: 0.33
Nodes (11): build_review_rows(), create_stock_history_csv(), import_history_csv(), portfolio_position_plan_document_fixture(), prepare_security_environment(), review_request(), security_envs(), security_post_trade_review_marks_open_position_as_pending_closeout() (+3 more)

### Community 50 - "Analysis Contextual"
Cohesion: 0.38
Nodes (9): build_choppy_history_rows(), build_confirmed_breakdown_rows(), build_confirmed_breakout_rows(), create_stock_history_csv(), import_history_csv(), security_analysis_contextual_keeps_mixed_when_stock_is_range_wait(), security_analysis_contextual_reports_headwind_when_stock_and_environment_conflict(), security_analysis_contextual_reports_tailwind_when_stock_market_sector_align() (+1 more)

### Community 51 - "Execution Journal"
Cohesion: 0.33
Nodes (10): approx_equal(), build_review_rows(), create_stock_history_csv(), execution_journal_request(), import_history_csv(), prepare_security_environment(), security_envs(), security_execution_journal_allows_open_position_snapshot_without_forced_flat_exit() (+2 more)

### Community 52 - "Create Runtime"
Cohesion: 0.21
Nodes (4): run_cli_with_bytes(), run_cli_with_json(), thread_runtime_db(), thread_runtime_root()

### Community 53 - "Committee Decision"
Cohesion: 0.55
Nodes (9): build_confirmed_breakout_rows(), committee_direction_tracks_final_action_when_majority_votes_avoid(), committee_needs_more_evidence_downgrades_action_to_abstain(), create_stock_history_csv(), import_history_csv(), security_decision_committee_blocks_trade_when_risk_reward_is_too_low(), security_decision_committee_returns_reviewable_or_deferred_outcome_when_evidence_and_risk_reward_align(), seven_seat_committee_exposes_member_opinions() (+1 more)

### Community 54 - "Validation Real"
Cohesion: 0.51
Nodes (9): build_sina_kline_body(), create_pool_proxy_history_fixture(), create_validation_runtime_root(), security_real_data_validation_backfill_auto_imports_cross_border_pool_proxy_history(), security_real_data_validation_backfill_enriches_treasury_etf_peer_environment(), security_real_data_validation_backfill_persists_price_history_context_and_manifest(), security_real_data_validation_backfill_preserves_equity_etf_native_profile_semantics(), security_real_data_validation_backfill_rejects_etf_slice_without_pool_proxy_history() (+1 more)

### Community 55 - "Position Plan"
Cohesion: 0.36
Nodes (8): build_confirmed_breakout_rows(), create_stock_history_csv(), import_history_csv(), position_plan_request(), prepare_security_environment(), security_envs(), security_position_plan_outputs_formal_document_aligned_with_briefing(), spawn_http_route_server()

### Community 56 - "Formal Boundary"
Cohesion: 0.4
Nodes (9): collect_rust_files(), declared_module_name(), declared_modules(), formal_boundary_manifest_gate_is_recorded_in_docs(), manifest_relative_paths(), normalize_newlines(), ops_root_keeps_only_stock_as_formal_boundary_in_split_repo(), stock_root_keeps_only_the_frozen_module_manifest() (+1 more)

### Community 57 - "Forward Outcome"
Cohesion: 0.39
Nodes (7): assert_float_eq(), build_linear_growth_rows(), create_stock_history_csv(), find_horizon(), import_history_csv(), security_forward_outcome_returns_snapshot_bound_multi_horizon_labels(), spawn_http_route_server()

### Community 58 - "Validation Create"
Cohesion: 0.42
Nodes (8): build_confirmed_breakout_rows(), create_json_fixture(), create_stock_history_csv(), create_validation_fixture_dir(), create_validation_runtime_db(), import_history_csv(), security_lifecycle_validation_slice_round_trips_formal_tools(), spawn_http_route_server()

### Community 59 - "Model Promotion"
Cohesion: 0.47
Nodes (7): create_promotion_fixture_dir(), security_model_promotion_emits_shadow_grade_decision_document(), security_model_promotion_only_upgrades_to_champion_after_stable_shadow_observations(), security_model_promotion_rejects_champion_when_oot_window_evidence_is_still_thin(), security_model_promotion_rejects_champion_when_shadow_observations_are_still_thin(), write_registry_fixture(), write_shadow_evaluation_fixture()

### Community 60 - "Position Contract"
Cohesion: 0.36
Nodes (5): approved_open_position_packet_document(), position_plan_document(), security_position_contract_caps_risk_budget_by_packet_single_trade_limit(), security_position_contract_falls_back_to_risk_grade_when_plan_budget_is_zero(), security_position_contract_rejects_symbol_mismatch_between_packet_and_plan()

### Community 61 - "Shadow Evaluation"
Cohesion: 0.53
Nodes (7): create_shadow_fixture_dir(), security_shadow_evaluation_builds_governed_shadow_readiness_document(), security_shadow_evaluation_tracks_repeated_shadow_observations_for_champion_readiness(), security_shadow_evaluation_tracks_window_and_oot_stability_for_champion_readiness(), write_history_expansion_fixture(), write_prior_shadow_evaluation_fixture(), write_registry_fixture()

### Community 62 - "History Build"
Cohesion: 0.42
Nodes (8): build_history_rows(), build_sina_kline_body(), create_stock_history_csv(), create_validation_runtime_root(), import_history_csv(), security_analysis_fullstack_prefers_governed_stock_history_before_live_fetch(), security_real_data_validation_backfill_persists_slice_local_stock_information_history(), spawn_http_route_server()

### Community 63 - "Decision Evidence"
Cohesion: 0.43
Nodes (6): build_choppy_history_rows(), build_confirmed_breakout_rows(), create_stock_history_csv(), import_history_csv(), security_decision_evidence_bundle_reports_analysis_date_and_data_gaps(), spawn_http_route_server()

### Community 64 - "History Scorecard"
Cohesion: 0.46
Nodes (7): build_history_rows(), create_stock_history_csv(), create_test_approval_root(), import_history_csv(), seeded_runtime_db(), spawn_http_route_server(), submit_approval_persists_formal_scorecard_even_without_model_artifact()

### Community 65 - "Post Meeting"
Cohesion: 0.48
Nodes (5): build_confirmed_breakout_rows(), create_stock_history_csv(), import_history_csv(), security_record_post_meeting_conclusion_creates_conclusion_and_revises_package(), spawn_http_route_server()

### Community 66 - "Entry Layer"
Cohesion: 0.52
Nodes (6): first_entry_layer_modules_stay_above_grouped_gateways(), governed_action_entry_keeps_governance_surface_complete(), normalize_newlines(), post_trade_learning_entry_keeps_training_composition_explicit(), research_sidecar_entry_keeps_sidecar_scope_explicit(), stock_boundary_exposes_first_entry_layer_modules()

### Community 67 - "Foundation Business"
Cohesion: 0.52
Nodes (6): collect_rust_files(), is_stock_business_file(), normalize_newlines(), stock_business_modules_do_not_import_generic_foundation_analytics(), stock_dispatcher_keeps_generic_foundation_analytics_outside_the_stock_bus(), stock_foundation_split_design_is_recorded_in_docs()

### Community 68 - "Resonance Trust"
Cohesion: 0.53
Nodes (4): build_etf_rows(), create_stock_history_csv(), import_history_csv(), security_etf_resonance_trust_pack_returns_current_verdict_and_replay_summary()

### Community 69 - "Legacy Committee"
Cohesion: 0.6
Nodes (5): collect_rs_files(), legacy_committee_application_surface_stays_explicitly_labeled(), legacy_committee_dispatcher_import_is_confined_to_one_explicit_file(), normalize_newlines(), normalize_rel_path()

### Community 70 - "Legacy Committee"
Cohesion: 0.6
Nodes (5): collect_rs_files(), key_business_callers_must_depend_on_compat_adapter(), legacy_committee_direct_dependency_is_confined_to_compat_adapter(), normalize_newlines(), normalize_rel_path()

### Community 71 - "Scorecard Refit"
Cohesion: 0.6
Nodes (4): create_refit_fixture_dir(), security_scorecard_refit_exposes_shadow_grade_when_promotion_decision_requests_it(), security_scorecard_refit_records_run_and_registers_candidate_artifact(), write_scorecard_model_artifact()

### Community 72 - "Training Data"
Cohesion: 0.6
Nodes (4): create_history_runtime_root(), spawn_query_aware_server(), stock_training_data_backfill_falls_back_to_sina_financial_and_announcement_history_when_eastmoney_fails(), stock_training_data_backfill_syncs_price_and_backfills_information_history_for_equities()

### Community 73 - "History Disclosure"
Cohesion: 0.6
Nodes (3): create_history_runtime_root(), security_disclosure_history_live_backfill_fetches_multiple_pages(), spawn_query_aware_server()

### Community 74 - "History Fundamental"
Cohesion: 0.6
Nodes (3): create_history_runtime_root(), security_fundamental_history_live_backfill_fetches_multiple_report_periods(), spawn_http_route_server()

### Community 75 - "History Expansion"
Cohesion: 0.4
Nodes (0): 

### Community 76 - "Catalog Grouped"
Cohesion: 0.7
Nodes (4): index_of(), normalize_newlines(), stock_catalog_keeps_grouped_business_sections_explicit(), stock_catalog_orders_formal_tools_by_grouped_business_flow()

### Community 77 - "Foundation Gate"
Cohesion: 0.7
Nodes (4): normalize_newlines(), shared_and_runtime_hold_zone_files_remain_present_and_shared(), split_manifest_and_gate_v2_baseline_are_recorded_in_docs(), stock_entry_and_grouped_shells_do_not_reach_foundation_or_hold_zone()

### Community 78 - "Execution Store"
Cohesion: 0.83
Nodes (3): open_security_execution_store_connection(), open_security_execution_store_connection_creates_parent_directory(), unique_temp_dir()

### Community 79 - "Decision Committee"
Cohesion: 0.83
Nodes (3): normalize_newlines(), security_decision_committee_stays_frozen_as_legacy_compatibility_zone(), stable_fnv1a64()

### Community 80 - "History Disclosure"
Cohesion: 0.67
Nodes (2): create_history_runtime_root(), security_disclosure_history_backfill_persists_recent_announcements()

### Community 81 - "Proxy External"
Cohesion: 0.67
Nodes (2): create_proxy_fixture_file(), security_external_proxy_history_import_reads_csv_and_persists_records()

### Community 82 - "History Fundamental"
Cohesion: 0.67
Nodes (2): create_history_runtime_root(), security_fundamental_history_backfill_persists_latest_report_snapshot()

### Community 83 - "Committee Catalog"
Cohesion: 0.83
Nodes (3): dispatcher_may_keep_legacy_committee_route_but_catalog_must_not(), normalize_newlines(), public_stock_catalog_promotes_formal_committee_mainline_only()

### Community 84 - "Position Portfolio"
Cohesion: 0.5
Nodes (0): 

### Community 85 - "Modeling Online"
Cohesion: 0.83
Nodes (3): normalize_newlines(), online_and_lifecycle_subgroups_keep_module_ownership_separate(), stock_modeling_gateway_declares_online_and_lifecycle_subgroups()

### Community 86 - "Securityposttradereviewassembler<'A> .Assemble()"
Cohesion: 0.67
Nodes (1): SecurityPostTradeReviewAssembler<'a>

### Community 87 - "Execution Bootstrap"
Cohesion: 1.0
Nodes (2): bootstrap_security_execution_schema(), bootstrap_security_execution_schema_creates_execution_tables()

### Community 88 - "Committee Vote"
Cohesion: 0.67
Nodes (0): 

### Community 89 - "Condition Review"
Cohesion: 0.67
Nodes (0): 

### Community 90 - "Decision Package"
Cohesion: 1.0
Nodes (2): decision_package_keeps_chair_resolution_as_explicit_object_graph_node(), normalize_newlines()

### Community 91 - "Verify Package"
Cohesion: 1.0
Nodes (2): normalize_newlines(), verify_package_keeps_post_meeting_chair_binding_visible()

### Community 92 - "External Proxy"
Cohesion: 0.67
Nodes (0): 

### Community 93 - "Independent Advice"
Cohesion: 0.67
Nodes (0): 

### Community 94 - "Dispatcher Grouping"
Cohesion: 1.0
Nodes (2): normalize_newlines(), stock_dispatcher_imports_grouped_gateways_for_formal_business_flows()

### Community 95 - "Lib.Rs Catalog"
Cohesion: 1.0
Nodes (0): 

### Community 96 - "Runtime Paths.Rs"
Cohesion: 1.0
Nodes (0): 

### Community 97 - "Support.Rs Lock"
Cohesion: 1.0
Nodes (0): 

### Community 98 - "Submit Approval"
Cohesion: 1.0
Nodes (0): 

### Community 99 - "Master Scorecard"
Cohesion: 1.0
Nodes (0): 

### Community 100 - "Community 100"
Cohesion: 1.0
Nodes (1): T

### Community 101 - "Stock.Rs"
Cohesion: 1.0
Nodes (0): 

## Knowledge Gaps
- **472 isolated node(s):** `ImportStockPriceHistoryRequest`, `ImportStockPriceHistoryResult`, `ImportDateRange`, `ImportStockPriceHistoryError`, `SecurityApprovedPortfolioCandidateInput` (+467 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Lib.Rs Catalog`** (2 nodes): `lib.rs`, `tool_catalog_json()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Runtime Paths.Rs`** (2 nodes): `runtime_paths.rs`, `workspace_runtime_dir()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Support.Rs Lock`** (2 nodes): `test_support.rs`, `lock_test_env()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Submit Approval`** (2 nodes): `security_decision_submit_approval_source_guard.rs`, `submit_approval_uses_formal_master_scorecard_mainline()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Master Scorecard`** (2 nodes): `security_master_scorecard_mainline_source_guard.rs`, `master_scorecard_result_keeps_composite_and_committee_payload_markers()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 100`** (1 nodes): `T`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Stock.Rs`** (1 nodes): `stock.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Are the 23 inferred relationships involving `build_consultation_result()` (e.g. with `technical_consultation_basic()` and `classify_trend_strength()`) actually correct?**
  _`build_consultation_result()` has 23 INFERRED edges - model-reasoned connections that need verification._
- **Are the 15 inferred relationships involving `build_security_portfolio_replacement_plan()` (e.g. with `security_portfolio_replacement_plan()` and `validate_request()`) actually correct?**
  _`build_security_portfolio_replacement_plan()` has 15 INFERRED edges - model-reasoned connections that need verification._
- **Are the 15 inferred relationships involving `build_indicator_snapshot()` (e.g. with `technical_consultation_basic()` and `ema_last()`) actually correct?**
  _`build_indicator_snapshot()` has 15 INFERRED edges - model-reasoned connections that need verification._
- **Are the 14 inferred relationships involving `security_analysis_fullstack()` (e.g. with `fetch_etf_context()` and `resolve_governed_etf_proxy_snapshot()`) actually correct?**
  _`security_analysis_fullstack()` has 14 INFERRED edges - model-reasoned connections that need verification._
- **Are the 14 inferred relationships involving `http_get_text()` (e.g. with `fetch_live_fundamental_history_rows_from_eastmoney()` and `fetch_live_fundamental_history_rows_from_sina()`) actually correct?**
  _`http_get_text()` has 14 INFERRED edges - model-reasoned connections that need verification._
- **What connects `ImportStockPriceHistoryRequest`, `ImportStockPriceHistoryResult`, `ImportDateRange` to the rest of the system?**
  _472 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Build Decision` be split into smaller, more focused modules?**
  _Cohesion score 0.02 - nodes in this community are weakly interconnected._

## Audit Limits

- This first repository-level graph audit was generated from Graphify AST extraction on code files only.
- Document, paper, and image semantic extraction was not included in this local run because the installed Graphify command wrapper does not expose the full corpus extraction pipeline directly in this environment.
- Use this audit as a structural repository map and closeout artifact, not as a complete cross-document semantic graph.
