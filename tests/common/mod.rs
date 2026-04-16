use assert_cmd::Command;
use rust_xlsxwriter::Workbook;
use serde_json::Value;
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::thread_local;
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    static THREAD_RUNTIME_DB: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

fn thread_runtime_db() -> PathBuf {
    THREAD_RUNTIME_DB.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some(path) = slot.as_ref() {
            return path.clone();
        }

        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let runtime_dir = PathBuf::from("tests")
            .join("runtime_fixtures")
            .join("thread_local_memory")
            .join(format!(
                "thread_{:?}_{unique_suffix}",
                std::thread::current().id()
            ));
        fs::create_dir_all(&runtime_dir).unwrap();
        let db_path = runtime_dir.join("runtime.db");
        *slot = Some(db_path.clone());
        db_path
    })
}

#[allow(dead_code)]
pub fn thread_runtime_root() -> PathBuf {
    thread_runtime_db()
        .parent()
        .expect("thread runtime db should always have a parent directory")
        .to_path_buf()
}

// 2026-03-21: ?????? CLI ????????????? Tool ????????????? JSON ????????
#[allow(dead_code)]
pub fn run_cli_with_json(input: &str) -> Value {
    let mut cmd = Command::cargo_bin("excel_skill").unwrap();
    cmd.env("EXCEL_SKILL_RUNTIME_DB", thread_runtime_db());
    let assert = cmd.write_stdin(input).assert().success();
    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    serde_json::from_str(&output).unwrap()
}

// 2026-03-22: ?????????? CLI ?????????? Windows ? UTF-8 ???????????????????
#[allow(dead_code)]
pub fn run_cli_with_bytes(input: Vec<u8>) -> Value {
    let mut cmd = Command::cargo_bin("excel_skill").unwrap();
    cmd.env("EXCEL_SKILL_RUNTIME_DB", thread_runtime_db());
    let assert = cmd.write_stdin(input).assert().success();
    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    serde_json::from_str(&output).unwrap()
}

// 2026-03-22: 这里补充带 runtime 路径的 CLI 测试助手，目的是让本地记忆层测试能隔离运行，避免不同测试共享同一个 SQLite 文件。
#[allow(dead_code)]
pub fn run_cli_with_json_and_runtime(input: &str, runtime_db_path: &PathBuf) -> Value {
    let mut cmd = Command::cargo_bin("excel_skill").unwrap();
    cmd.env("EXCEL_SKILL_RUNTIME_DB", runtime_db_path);
    let assert = cmd.write_stdin(input).assert().success();
    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    serde_json::from_str(&output).unwrap()
}

// 2026-03-29 CST: 这里补充带额外环境变量的 CLI 测试助手，原因是股票 HTTP 同步测试需要把本地假服务地址注入到子进程；
// 目的：让测试能稳定隔离腾讯/新浪 provider，而不依赖真实外网或硬编码线上地址。
#[allow(dead_code)]
pub fn run_cli_with_json_runtime_and_envs(
    input: &str,
    runtime_db_path: &PathBuf,
    envs: &[(&str, String)],
) -> Value {
    let mut cmd = Command::cargo_bin("excel_skill").unwrap();
    cmd.env("EXCEL_SKILL_RUNTIME_DB", runtime_db_path);
    for (key, value) in envs {
        cmd.env(key, value);
    }
    let assert = cmd.write_stdin(input).assert().success();
    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    serde_json::from_str(&output).unwrap()
}

// 2026-03-22: ?????????????????????? Windows ??????????????????????
#[allow(dead_code)]
pub fn create_chinese_path_fixture(file_name: &str) -> PathBuf {
    let base_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("\u{4e2d}\u{6587}\u{8def}\u{5f84}")
        .join("\u{65b0}\u{7586}\u{5ba2}\u{6237}");
    fs::create_dir_all(&base_dir).unwrap();

    let target_path = base_dir.join(file_name);
    fs::copy("tests/fixtures/basic-sales.xlsx", &target_path).unwrap();
    target_path
}

// 2026-03-22: 这里统一生成测试专用 runtime SQLite 路径，目的是把跨请求状态测试限制在各自独立目录下，减少并发串扰。
#[allow(dead_code)]
pub fn create_test_runtime_db(prefix: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let runtime_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("local_memory")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&runtime_dir).unwrap();
    runtime_dir.join("runtime.db")
}

// 2026-03-22: 这里统一生成测试导出路径，目的是把 CSV/XLSX 导出测试限制在独立目录下，避免不同测试互相覆盖工件。
#[allow(dead_code)]
pub fn create_test_output_path(prefix: &str, extension: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let export_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("exports")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&export_dir).unwrap();
    export_dir.join(format!("output.{extension}"))
}

// 2026-03-23: 这里统一生成测试专用工作簿，目的是为需要运行时构造 Base / Lookup 多 sheet 场景的 CLI 测试提供稳定夹具。
#[allow(dead_code)]
pub fn create_test_workbook(
    prefix: &str,
    workbook_name: &str,
    sheets: &[(&str, Vec<Vec<&str>>)],
) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let workbook_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("generated_workbooks")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&workbook_dir).unwrap();

    let output_path = workbook_dir.join(workbook_name);
    let mut workbook = Workbook::new();

    for (sheet_name, rows) in sheets {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(*sheet_name).unwrap();

        for (row_index, row) in rows.iter().enumerate() {
            for (column_index, value) in row.iter().enumerate() {
                worksheet
                    .write_string(row_index as u32, column_index as u16, *value)
                    .unwrap();
            }
        }
    }

    workbook.save(&output_path).unwrap();
    output_path
}

// 2026-03-22: 这里补充显式单元格坐标写入助手，目的是为区域探查/区域加载测试稳定构造“不从 A1 开始”的工作表。
#[allow(dead_code)]
pub fn create_positioned_workbook(
    prefix: &str,
    workbook_name: &str,
    sheets: &[(&str, Vec<(u32, u16, &str)>)],
) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let workbook_dir = PathBuf::from("tests")
        .join("runtime_fixtures")
        .join("generated_workbooks")
        .join(format!("{prefix}_{unique_suffix}"));
    fs::create_dir_all(&workbook_dir).unwrap();

    let output_path = workbook_dir.join(workbook_name);
    let mut workbook = Workbook::new();

    for (sheet_name, cells) in sheets {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(*sheet_name).unwrap();

        for (row_index, column_index, value) in cells {
            worksheet
                .write_string(*row_index, *column_index, *value)
                .unwrap();
        }
    }

    workbook.save(&output_path).unwrap();
    output_path
}
