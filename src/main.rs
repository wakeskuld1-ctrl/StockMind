use std::io::{self, Read};

use encoding_rs::GBK;
use excel_skill::tool_catalog_json;
use excel_skill::tools::contracts::{ToolRequest, ToolResponse};
use excel_skill::tools::dispatcher::dispatch;

fn main() {
    let mut input_bytes = Vec::new();
    match io::stdin().read_to_end(&mut input_bytes) {
        Ok(_) => {
            let input = match decode_input_bytes(&input_bytes) {
                Ok(input) => input,
                Err(error) => {
                    print_response(ToolResponse::error(error));
                    return;
                }
            };

            if input.trim().is_empty() {
                println!("{}", tool_catalog_json());
                return;
            }

            print_response(handle_request_input(input));
        }
        Err(error) => print_response(ToolResponse::error(format!(
            "\u{8bfb}\u{53d6}\u{6807}\u{51c6}\u{8f93}\u{5165}\u{5931}\u{8d25}: {error}"
        ))),
    }
}

fn handle_request_input(input: String) -> ToolResponse {
    match serde_json::from_str::<ToolRequest>(&input) {
        Ok(request) => handle_tool_request(request),
        Err(error) => ToolResponse::error(format!(
            "\u{8bf7}\u{6c42} JSON \u{89e3}\u{6790}\u{5931}\u{8d25}: {error}"
        )),
    }
}

fn handle_tool_request(request: ToolRequest) -> ToolResponse {
    // 2026-04-16 CST: Added because the split repo keeps only the stock tool bus.
    // Reason: the original product-level license shell is intentionally excluded from StockMind.
    // Purpose: route requests directly into the stock dispatcher without extra gating layers.
    dispatch(request)
}

fn decode_input_bytes(input_bytes: &[u8]) -> Result<String, String> {
    if input_bytes.is_empty() {
        return Ok(String::new());
    }

    if let Some(decoded) = decode_utf8_with_optional_bom(input_bytes) {
        return Ok(decoded);
    }

    if let Some(decoded) = decode_utf16_with_bom(input_bytes) {
        return Ok(decoded);
    }

    let (decoded, _, had_errors) = GBK.decode(input_bytes);
    if !had_errors {
        return Ok(decoded.into_owned());
    }

    Err(
        "\u{6807}\u{51c6}\u{8f93}\u{5165}\u{4e0d}\u{662f}\u{53ef}\u{8bc6}\u{522b}\u{7684} UTF-8 / UTF-16 / GBK \u{7f16}\u{7801}\u{ff0c}\u{65e0}\u{6cd5}\u{89e3}\u{6790}\u{8bf7}\u{6c42}".to_string(),
    )
}

fn decode_utf8_with_optional_bom(input_bytes: &[u8]) -> Option<String> {
    const UTF8_BOM: &[u8; 3] = b"\xEF\xBB\xBF";
    let bytes = input_bytes.strip_prefix(UTF8_BOM).unwrap_or(input_bytes);
    String::from_utf8(bytes.to_vec()).ok()
}

fn decode_utf16_with_bom(input_bytes: &[u8]) -> Option<String> {
    const UTF16_LE_BOM: &[u8; 2] = b"\xFF\xFE";
    const UTF16_BE_BOM: &[u8; 2] = b"\xFE\xFF";

    if let Some(bytes) = input_bytes.strip_prefix(UTF16_LE_BOM) {
        return decode_utf16_units(bytes, true);
    }
    if let Some(bytes) = input_bytes.strip_prefix(UTF16_BE_BOM) {
        return decode_utf16_units(bytes, false);
    }

    None
}

fn decode_utf16_units(bytes: &[u8], little_endian: bool) -> Option<String> {
    if bytes.len() % 2 != 0 {
        return None;
    }

    let units = bytes
        .chunks_exact(2)
        .map(|chunk| {
            if little_endian {
                u16::from_le_bytes([chunk[0], chunk[1]])
            } else {
                u16::from_be_bytes([chunk[0], chunk[1]])
            }
        })
        .collect::<Vec<_>>();

    String::from_utf16(&units).ok()
}

fn print_response(response: ToolResponse) {
    let payload =
        serde_json::to_string(&response).expect("tool response serialization should succeed");
    println!("{}", payload);
}
