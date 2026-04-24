// 2026-04-02 CST: Reworked the stock boundary declaration; reason: the repository already formed separate foundation and stock business lines; purpose: keep stock-domain exports on one explicit gateway instead of repeatedly patching module declarations. // 2026-04-15 CST: Updated because stock.rs is the only formal stock-domain gateway during the refactor; purpose: remove wording that suggested multiple official paths.
// Formal stock-domain mainline modules.
// 2026-04-15 CST: Added because the application-layer regrouping now needs explicit
// business-stage gateway modules on top of the unchanged flat stock entry surface.
// Reason: we need one stable grouping layer without breaking existing
// `crate::ops::stock::<module>` call paths during the refactor.
// Purpose: let later cleanup move callers toward business-stage groupings while
// preserving the current formal stock boundary.
#[path = "stock_data_pipeline.rs"]
pub mod stock_data_pipeline;

#[path = "stock_pre_trade.rs"]
pub mod stock_pre_trade;

#[path = "stock_governance_and_positioning.rs"]
pub mod stock_governance_and_positioning;

#[path = "stock_execution_and_position_management.rs"]
pub mod stock_execution_and_position_management;

#[path = "stock_post_trade.rs"]
pub mod stock_post_trade;

#[path = "stock_modeling_and_training.rs"]
pub mod stock_modeling_and_training;

#[path = "stock_research_sidecar.rs"]
pub mod stock_research_sidecar;

// 2026-04-15 CST: Added because the third-layer stock application entry architecture
// now starts landing above the grouped gateway layer.
// Reason: grouped gateways describe capability families, but later AI sessions also
// need explicit scenario-entry modules for stable business starting points.
// Purpose: expose the first stable entry modules without breaking the unchanged
// grouped gateway layer or the older flat stock boundary.
#[path = "stock_data_readiness_entry.rs"]
pub mod stock_data_readiness_entry;

#[path = "stock_investment_case_entry.rs"]
pub mod stock_investment_case_entry;

#[path = "stock_governed_action_entry.rs"]
pub mod stock_governed_action_entry;

// 2026-04-16 CST: Added because the approved third-layer stock application architecture
// now needs the in-trade scenario entry above grouped execution capabilities.
// Reason: without one explicit position-management entry, later AI sessions can still
// bypass the scenario layer and treat execution plus open-position logic as flat tools.
// Purpose: expose the formal mid-trade entry without changing the grouped gateway or
// reopening runtime-facing ownership.
#[path = "stock_position_management_entry.rs"]
pub mod stock_position_management_entry;

// 2026-04-16 CST: Added because the approved third-layer stock application architecture
// now needs the post-trade closure and learning scenario entry.
// Reason: review and training already form one feedback stage in the design baseline,
// but the code boundary still lacked one explicit scenario shell for that stage.
// Purpose: expose the formal post-trade learning entry without changing any runtime or
// grouped-gateway contracts.
#[path = "stock_post_trade_learning_entry.rs"]
pub mod stock_post_trade_learning_entry;

// 2026-04-16 CST: Added because the approved third-layer stock application architecture
// already reserved one explicit research-sidecar scenario entry above grouped sidecar capabilities.
// Reason: landing the rest of the entry layer without the sidecar shell left a real
// design/code gap in the formal stock boundary.
// Purpose: close the scenario-entry set while keeping research flows clearly outside
// the formal production mainline.
#[path = "stock_research_sidecar_entry.rs"]
pub mod stock_research_sidecar_entry;

#[path = "import_stock_price_history.rs"]
pub mod import_stock_price_history;

#[path = "security_analysis_contextual.rs"]
pub mod security_analysis_contextual;

#[path = "security_analysis_fullstack.rs"]
pub mod security_analysis_fullstack;

#[path = "security_independent_advice.rs"]
pub mod security_independent_advice;

// 2026-04-09 CST: 这里接入新的仓位与执行主链模块，原因是 4-9 起 position_plan / execution_record / journal 已经成为正式 Tool。
// 目的：让标准化交付能力沿单票、账户、执行与复盘主线持续演进，同时保留旧对象做平滑过渡。
#[path = "security_position_plan.rs"]
pub mod security_position_plan;

#[path = "security_portfolio_position_plan.rs"]
pub mod security_portfolio_position_plan;

#[path = "security_post_trade_review.rs"]
pub mod security_post_trade_review;

#[path = "security_execution_record.rs"]
pub mod security_execution_record;

#[path = "security_execution_journal.rs"]
pub mod security_execution_journal;

// 2026-04-18 CST: Added because Task 1 introduces the only formal approved
// post-open intake packet before any downstream contract or monitoring objects land.
// Reason: the user approved keeping post-open math and governance evidence on a
// pure data path that starts from one stable approved packet.
// Purpose: expose the approved intake contract on the stock boundary while later
// grouped gateways keep ownership of the in-trade loop semantics.
#[path = "security_approved_open_position_packet.rs"]
pub mod security_approved_open_position_packet;

// 2026-04-19 CST: Added because Task 1 of P10-P12 now lands the formal
// account-level objective shell above the existing post-open position objects.
// Reason: the portfolio-core expansion starts by freezing one governed account
// optimization problem before any unified replacement solver is added.
// Purpose: expose the account objective contract on the formal stock boundary.
#[path = "security_account_objective_contract.rs"]
pub mod security_account_objective_contract;

// 2026-04-19 CST: Added because Task 3 now lands the first formal P11
// unified replacement solver above the P10 objective and candidate set.
// Reason: the approved stage plan separates the replacement solve from both
// account-objective normalization and later allocation decision freeze.
// Purpose: expose the portfolio replacement plan on the formal stock boundary.
#[path = "security_portfolio_replacement_plan.rs"]
pub mod security_portfolio_replacement_plan;

// 2026-04-20 CST: Added because P12 now lands the minimum governed final
// allocation decision layer above the implemented P10 and P11 contracts.
// Reason: the approved route completes the portfolio-core chain with one
// explicit decision freeze instead of another solver pass.
// Purpose: expose the portfolio allocation decision on the formal stock boundary.
#[path = "security_portfolio_allocation_decision.rs"]
pub mod security_portfolio_allocation_decision;

// 2026-04-20 CST: Added because the next approved downstream step after P12 is
// one execution-preview bridge, not direct runtime execution.
// Reason: the user approved a side-effect-free consumer of the governed
// allocation decision before any new execution-writing stage.
// Purpose: expose the portfolio execution preview on the formal stock boundary.
#[path = "security_portfolio_execution_preview.rs"]
pub mod security_portfolio_execution_preview;

// 2026-04-20 CST: Added because P13 now introduces the first formal request
// bridge downstream of the standardized preview document.
// Reason: the approved next step after preview standardization is request
// packaging, not direct runtime execution or an approval detour.
// Purpose: expose the portfolio execution request package on the formal stock boundary.
#[path = "security_portfolio_execution_request_package.rs"]
pub mod security_portfolio_execution_request_package;

// 2026-04-21 CST: Added because P14 now introduces one explicit enrichment
// bridge downstream of the formal P13 request package and upstream of any apply stage.
// Reason: the approved route keeps request enrichment separate from runtime execution facts.
// Purpose: expose the portfolio execution request enrichment bridge on the formal stock boundary.
#[path = "security_portfolio_execution_request_enrichment.rs"]
pub mod security_portfolio_execution_request_enrichment;

// 2026-04-21 CST: Added because P15 now introduces the first governed apply
// bridge downstream of the formal P14 enrichment bundle.
// Reason: the approved route reuses the existing execution-record mainline
// instead of treating apply as implicit downstream glue.
// Purpose: expose the portfolio execution apply bridge on the formal stock boundary.
#[path = "security_portfolio_execution_apply_bridge.rs"]
pub mod security_portfolio_execution_apply_bridge;

// 2026-04-22 CST: Added because P16 now introduces one pure execution-status
// freeze layer immediately after the governed P15 apply bridge.
// Reason: the approved route keeps status freezing separate from later
// reconciliation and position-materialization work.
// Purpose: expose the portfolio execution status bridge on the formal stock boundary.
#[path = "security_portfolio_execution_status_bridge.rs"]
pub mod security_portfolio_execution_status_bridge;

// 2026-04-18 CST: Added because Task 2 now lands the only formal live
// governance object between approved intake and active holdings.
// Reason: the user fixed `PositionContract` as a new object that must stay
// distinct from the pre-trade position-plan document.
// Purpose: expose the post-open contract module on the stock boundary.
#[path = "security_position_contract.rs"]
pub mod security_position_contract;

// 2026-04-18 CST: Added because Task 4 now lands the formal single-position
// evaluation object between the active book and later monitoring evidence.
// Reason: the approved post-open data flow requires a dedicated evaluation layer
// instead of folding action scoring into snapshot or aggregation logic.
// Purpose: expose the per-position evaluation module on the stock boundary.
#[path = "security_per_position_evaluation.rs"]
pub mod security_per_position_evaluation;

// 2026-04-18 CST: Added because Task 5 now lands the standardized monitoring
// evidence package above per-position evaluations.
// Reason: the approved post-open data flow needs one formal committee-facing
// evidence artifact before later simulation and governance packaging steps.
// Purpose: expose the monitoring evidence package module on the stock boundary.
#[path = "security_monitoring_evidence_package.rs"]
pub mod security_monitoring_evidence_package;

// 2026-04-19 CST: Added because Task 6 now lands the account-level capital
// rebasing chain as its own formal post-open module.
// Reason: capital events must stay separate from ordinary add/trim logic and
// should not be hidden inside monitoring or execution files.
// Purpose: expose the capital rebase module on the stock boundary.
#[path = "security_capital_rebase.rs"]
pub mod security_capital_rebase;

// 2026-04-14 CST: Added because round 2 plan B now starts extracting the formal
// execution-record builder layer into dedicated internal modules.
// Purpose: let the stock boundary host the new formal assembler/resolver slices
// without promoting them as public top-level entrypoints.
#[path = "security_execution_record_assembler.rs"]
pub(crate) mod security_execution_record_assembler;

#[path = "security_execution_account_binding_resolver.rs"]
pub(crate) mod security_execution_account_binding_resolver;

#[path = "security_account_open_position_snapshot_assembler.rs"]
pub(crate) mod security_account_open_position_snapshot_assembler;

#[path = "security_open_position_corporate_action_summary.rs"]
pub(crate) mod security_open_position_corporate_action_summary;

#[path = "security_post_trade_review_assembler.rs"]
pub(crate) mod security_post_trade_review_assembler;

#[path = "security_post_trade_review_policy.rs"]
pub(crate) mod security_post_trade_review_policy;

// 2026-04-10 CST: 这里新增账户 open snapshot 对象模块，原因是方案B要求把“上一轮 execution_record -> 下一轮账户输入”独立收口；
// 目的：让账户层自动回接拥有稳定的正式对象入口，而不是继续写进 portfolio plan 内部隐式逻辑。
#[path = "security_account_open_position_snapshot.rs"]
pub mod security_account_open_position_snapshot;

// 2026-04-09 CST: 这里新增统一日期/补数门禁辅助模块，原因是用户要求把“本地优先 -> 自动补数 -> 最近交易日回退 -> 显式日期说明”内建到 Tool/Contract。
// 目的：供 technical/fullstack/briefing/position_plan 同源复用，避免规则继续只停留在 Skill 文档口头层。
#[path = "stock_analysis_data_guard.rs"]
pub mod stock_analysis_data_guard;

#[path = "technical_consultation_basic.rs"]
pub mod technical_consultation_basic;

#[path = "security_decision_evidence_bundle.rs"]
pub mod security_decision_evidence_bundle;

// 2026-04-17 CST: Added because the full scorecard-training migration now also depends on the
// governed symbol taxonomy module used by snapshot, runtime scorecard, and training together.
// Reason: without exporting the shared routing helper, the migrated thicker training chain would
// compile against a module path that does not exist in the split repo.
// Purpose: keep symbol-level market/sector routing inside the stock boundary as one internal tool.
#[path = "security_symbol_taxonomy.rs"]
pub(crate) mod security_symbol_taxonomy;

// 2026-04-16 CST: Added because plan A now starts landing the formal composite scorecard
// business object on the stock domain boundary.
// Reason: the approved core-value design needs one stable code artifact before any deeper
// committee or chair rewiring happens.
// Purpose: expose the new composite scorecard module through the single stock-domain boundary.
#[path = "security_composite_scorecard.rs"]
pub mod security_composite_scorecard;

// 2026-04-16 CST: Added because approved plan A now needs the formal adapter that projects
// the composite business object into the governed committee payload contract.
// Reason: this bridge must be visible on the single stock-domain boundary before the master
// scorecard mainline can return it.
// Purpose: expose the adapter through the existing stock gateway without changing runtime paths.
#[path = "security_composite_committee_payload_adapter.rs"]
pub mod security_composite_committee_payload_adapter;

// 2026-04-15 CST: Added because the ETF trust-pack is now a formal stock-domain entrypoint.
// Reason: ETF decision trust must live on the same governed securities surface as evidence and replay tools.
// Purpose: expose the current-evidence plus historical-replay ETF validator through the official stock boundary.
#[path = "security_etf_resonance_trust_pack.rs"]
pub mod security_etf_resonance_trust_pack;

#[path = "security_risk_gates.rs"]
pub mod security_risk_gates;

#[path = "security_decision_card.rs"]
pub mod security_decision_card;

#[path = "security_decision_committee.rs"]
pub mod security_decision_committee;

// 2026-04-16 CST: Added because the legacy committee freeze now also needs one
// controlled compatibility adapter on the formal stock boundary.
// Reason: downstream business modules still need the old contract temporarily,
// but should no longer import the frozen legacy module directly.
// Purpose: expose one transition seam for later retirement without reopening a
// second committee engine or widening the refactor surface.
#[path = "security_legacy_committee_compat.rs"]
pub(crate) mod security_legacy_committee_compat;

#[path = "security_scorecard.rs"]
pub mod security_scorecard;

// 2026-04-14 CST: 这里补回主评分卡汇总导出，原因是 position_plan / approval_brief /
// submit_approval 当前都已经直接消费 master scorecard 文档。
// 目的：让证券主链继续以 stock.rs 为单一业务边界，不再因为漏导出导致调用方误判“模块不存在”。
#[path = "security_master_scorecard.rs"]
pub mod security_master_scorecard;

// 2026-04-14 CST: 这里补回模型晋升导出，原因是 submit_approval 已经把模型治理摘要纳入正式 package。
// 目的：恢复当前证券主链对 promotion 文档的正式可见性，避免 stock 域导出和真实模块继续失配。
#[path = "security_model_promotion.rs"]
pub mod security_model_promotion;

// 2026-04-14 CST: 这里补回 shadow evaluation 导出，原因是模型治理摘要与审批链仍直接引用 shadow 文档。
// 目的：保持 stock 边界与现有 submit_approval / package 读取逻辑一致，先做最小收口。
#[path = "security_shadow_evaluation.rs"]
pub mod security_shadow_evaluation;

#[path = "security_chair_resolution.rs"]
pub mod security_chair_resolution;

#[path = "security_record_post_meeting_conclusion.rs"]
pub mod security_record_post_meeting_conclusion;

// 2026-04-14 CST: Added because the live backfill bridge depends on the governed persistence layer
// already implemented in the stock domain, but that layer was not exported through stock.rs.
// Purpose: keep live import and governed storage on one explicit stock boundary.
#[path = "security_fundamental_history_backfill.rs"]
pub mod security_fundamental_history_backfill;

// 2026-04-14 CST: Added because governed disclosure persistence must be visible from the stock
// boundary before the live bridge and later batch backfill can reuse it.
// Purpose: expose stock disclosure-history storage on the same formal domain surface.
#[path = "security_disclosure_history_backfill.rs"]
pub mod security_disclosure_history_backfill;

// 2026-04-14 CST: Added because plan A+ needs governed financial history backfill to become a
// formal stock-domain entrypoint instead of staying as an isolated implementation file.
// Purpose: expose stock financial-history live backfill through the same stock boundary used by CLI and Skills.
#[path = "security_fundamental_history_live_backfill.rs"]
pub mod security_fundamental_history_live_backfill;

// 2026-04-14 CST: Added because plan A+ also needs governed disclosure history to enter the stock
// mainline through a stable domain gateway rather than ad-hoc direct module calls.
// Purpose: make announcement-history live backfill discoverable and routable from the stock boundary.
#[path = "security_disclosure_history_live_backfill.rs"]
pub mod security_disclosure_history_live_backfill;

// 2026-04-18 CST: Added because scheme C2 now needs a formal corporate-action
// backfill entry on the stock boundary before training-data completion starts.
// Purpose: expose governed dividend and bonus/split import through the same
// stock-domain gateway as the other dated history backfill tools.
#[path = "security_corporate_action_backfill.rs"]
pub mod security_corporate_action_backfill;

// 2026-04-10 CST: 这里补挂轻量会后结论对象模块，原因是方案A要先把远端缺失的独立对象能力最小并回，
// 目的：在保留现有 record_post_meeting_conclusion 主链不动的前提下，补齐新的 post_meeting_conclusion 合同层。
#[path = "security_post_meeting_conclusion.rs"]
pub mod security_post_meeting_conclusion;

#[path = "security_decision_package.rs"]
pub mod security_decision_package;

#[path = "security_decision_verify_package.rs"]
pub mod security_decision_verify_package;

#[path = "security_decision_package_revision.rs"]
pub mod security_decision_package_revision;

#[path = "security_feature_snapshot.rs"]
pub mod security_feature_snapshot;

#[path = "security_forward_outcome.rs"]
pub mod security_forward_outcome;

// 2026-04-14 CST: 这里补回 external proxy backfill 模块导出，原因是 history_expansion 当前仍通过 stock 边界引用该正式能力；
// 目的：先恢复证券主链对 dated proxy backfill 的可见性，避免编译阶段误判为“模块不存在”而不是“实现未对齐”。
#[path = "security_external_proxy_backfill.rs"]
pub mod security_external_proxy_backfill;

// 2026-04-22 CST: Added because Task 1 lands the governed capital-flow backfill
// tool on the formal stock boundary before any JPX/MOF fetch adapters exist.
// Reason: dispatcher and grouped data-pipeline surfaces need one stable module
// export instead of reaching the op file through ad-hoc private paths.
// Purpose: expose the approved raw capital-flow ingestion contract without
// widening the change surface into factor generation or training.
#[path = "security_capital_flow_backfill.rs"]
pub mod security_capital_flow_backfill;
// 2026-04-22 CST: Added because the user requested one direct raw-source audit
// layer before any ratio or training interpretation continues.
// Purpose: expose governed JPX/MOF raw weekly table rendering on the formal stock boundary.
#[path = "security_capital_flow_raw_audit.rs"]
pub mod security_capital_flow_raw_audit;

// 2026-04-21 CST: Added because Task 2.1 now needs one JPX-specific workbook
// import bridge above the governed capital-flow backfill contract.
// Purpose: expose the approved JPX weekly investor-type import path on the formal stock boundary.
#[path = "security_capital_flow_jpx_weekly_import.rs"]
pub mod security_capital_flow_jpx_weekly_import;
// 2026-04-22 CST: Added because the approved source-supplement route now needs
// one live JPX archive bridge above the single-file workbook import contract.
// Purpose: expose multi-week official JPX raw backfill on the formal stock boundary.
#[path = "security_capital_flow_jpx_weekly_live_backfill.rs"]
pub mod security_capital_flow_jpx_weekly_live_backfill;

// 2026-04-21 CST: Added because Task 2.2 now needs one MOF-specific weekly CSV
// import bridge above the governed capital-flow backfill contract.
// Purpose: expose the approved MOF weekly cross-border import path on the formal stock boundary.
#[path = "security_capital_flow_mof_weekly_import.rs"]
pub mod security_capital_flow_mof_weekly_import;
// 2026-04-22 CST: Added because scheme A now needs one standalone factor layer
// above governed JPX/MOF raw flows and below any audit or training merge decision.
// Purpose: expose the capital-source factor snapshot on the formal stock boundary.
#[path = "security_capital_source_factor_snapshot.rs"]
pub mod security_capital_source_factor_snapshot;
// 2026-04-22 CST: Added because scheme A also needs one standalone factor-audit
// route before any capital-source feature is allowed near the main trainer.
// Purpose: expose factor-level backtesting on the formal stock boundary.
#[path = "security_capital_source_factor_audit.rs"]
pub mod security_capital_source_factor_audit;

// 2026-04-14 CST: 这里补回历史扩容模块导出，原因是 shadow evaluation 仍直接依赖
// history expansion 文档来判断样本覆盖和晋升准备度。
// 目的：先恢复模型治理链路的模块可见性，避免因为 stock 边界漏导出而误判实现缺失。
#[path = "security_history_expansion.rs"]
pub mod security_history_expansion;

#[path = "security_scorecard_model_registry.rs"]
pub mod security_scorecard_model_registry;

#[path = "security_scorecard_refit_run.rs"]
pub mod security_scorecard_refit_run;

// 2026-04-09 CST: 这里挂入正式训练入口模块，原因是 Task 5 需要把离线 scorecard 训练纳入证券主链边界；
// 目的：让训练能力与 snapshot、forward_outcome、refit 处于同一 stock 域内持续演进，避免回退到脚本式管理。
#[path = "security_scorecard_training.rs"]
pub mod security_scorecard_training;

// 2026-04-10 CST: 这里补挂审批后续与条件复核模块，原因是远端证券主线已经新增 approval brief / submit approval / condition review，
// 目的：在不整分支合并的前提下，把当前分支缺失的正式审批链入口精确并回 stock 业务域。
#[path = "security_approval_brief_signature.rs"]
pub mod security_approval_brief_signature;

#[path = "security_decision_approval_bridge.rs"]
pub mod security_decision_approval_bridge;

#[path = "security_decision_approval_brief.rs"]
pub mod security_decision_approval_brief;

#[path = "security_condition_review.rs"]
pub mod security_condition_review;

#[path = "security_decision_submit_approval.rs"]
pub mod security_decision_submit_approval;

// 2026-04-14 CST: Added because these modules are still referenced from the current stock surface; purpose: keep supporting adapters visible inside the domain boundary without advertising them as a second formal path. // 2026-04-15 CST: Updated because later AI sessions must read these as supporting modules under one mainline, not as a parallel transition track.
// Supporting stock-domain modules kept inside the same boundary.
#[path = "security_decision_briefing.rs"]
pub mod security_decision_briefing;

// 2026-04-16 CST: Added because the current stock surface already contains callers that rely on
// the governed file-based ETF proxy import bridge.
// Reason: the module file exists, but the stock boundary was not exporting it, which breaks
// downstream compile-time resolution.
// Purpose: restore the missing export without widening the public product surface beyond the
// already-existing implementation module.
#[path = "security_external_proxy_history_import.rs"]
pub mod security_external_proxy_history_import;

#[path = "security_position_plan_record.rs"]
pub mod security_position_plan_record;

#[path = "security_record_position_adjustment.rs"]
pub mod security_record_position_adjustment;

// 2026-04-14 CST: Added because resonance and signal-outcome logic remains exploratory and should stay visible without being mistaken for the formal execution path; purpose: preserve research visibility while keeping one governed stock mainline. // 2026-04-15 CST: Updated because later AI sessions must treat these as research-only modules rather than a second stock track.
// Research-only modules kept visible from the stock boundary.
#[path = "security_committee_vote.rs"]
pub mod security_committee_vote;

#[path = "security_analysis_resonance.rs"]
pub mod security_analysis_resonance;

#[path = "sync_template_resonance_factors.rs"]
pub mod sync_template_resonance_factors;

#[path = "signal_outcome_research.rs"]
pub mod signal_outcome_research;

#[path = "sync_stock_price_history.rs"]
pub mod sync_stock_price_history;

// 2026-04-14 CST: Added because plan A+ needs one formal stock-domain batch operation to thicken
// retraining data without hand-chaining separate price and information-history tools.
// Purpose: expose the minimal stock-only data-preparation batch from the stock boundary.
#[path = "stock_training_data_backfill.rs"]
pub mod stock_training_data_backfill;

// 2026-04-14 CST: Added because stock-first real-trading readiness now needs a formal
// coverage gate after backfill and before retraining.
// Purpose: expose the minimal stock-pool coverage audit from the stock domain boundary.
#[path = "stock_training_data_coverage_audit.rs"]
pub mod stock_training_data_coverage_audit;

// 2026-04-16 CST: Added because the formal boundary gate exposed that the governed
// validation-slice backfill had fallen out of the stock manifest while docs/tests
// still treated it as a formal stock capability.
// Purpose: restore one explicit stock-domain entrypoint for slice-local real-data
// validation replay instead of leaving a hidden stock/runtime bridge outside the manifest.
#[path = "security_real_data_validation_backfill.rs"]
pub mod security_real_data_validation_backfill;
