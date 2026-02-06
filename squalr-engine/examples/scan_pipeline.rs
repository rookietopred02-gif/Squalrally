use squalr_engine::engine_mode::EngineMode;
use squalr_engine::squalr_engine::SqualrEngine;
use squalr_engine_api::commands::privileged_command_request::PrivilegedCommandRequest;
use squalr_engine_api::commands::process::open::process_open_request::ProcessOpenRequest;
use squalr_engine_api::commands::scan::element_scan::element_scan_request::ElementScanRequest;
use squalr_engine_api::commands::scan::new::scan_new_request::ScanNewRequest;
use squalr_engine_api::commands::scan_results::query::scan_results_query_request::ScanResultsQueryRequest;
use squalr_engine_api::events::scan_results::updated::scan_results_updated_event::ScanResultsUpdatedEvent;
use squalr_engine_api::structures::data_types::built_in_types::i32::data_type_i32::DataTypeI32;
use squalr_engine_api::structures::data_types::data_type_ref::DataTypeRef;
use squalr_engine_api::structures::data_values::anonymous_value_string::AnonymousValueString;
use squalr_engine_api::structures::data_values::anonymous_value_string_format::AnonymousValueStringFormat;
use squalr_engine_api::structures::data_values::container_type::ContainerType;
use squalr_engine_api::structures::scanning::comparisons::scan_compare_type::ScanCompareType;
use squalr_engine_api::structures::scanning::comparisons::scan_compare_type_immediate::ScanCompareTypeImmediate;
use squalr_engine_api::structures::scanning::constraints::anonymous_scan_constraint::AnonymousScanConstraint;
use std::sync::mpsc;
use std::time::Duration;

fn main() {
    let pid: u32 = std::env::args()
        .nth(1)
        .expect("usage: scan_pipeline <pid>")
        .parse()
        .expect("pid must be u32");

    let mut engine = SqualrEngine::new(EngineMode::Standalone).expect("engine init failed");
    engine.initialize();

    let engine_unprivileged_state = engine
        .get_engine_unprivileged_state()
        .as_ref()
        .expect("no unprivileged state")
        .clone();
    let engine_privileged_state = engine
        .get_engine_privileged_state()
        .as_ref()
        .expect("no privileged state")
        .clone();

    // Open process.
    {
        let (tx, rx) = mpsc::channel();
        let req = ProcessOpenRequest {
            process_id: Some(pid),
            search_name: None,
            match_case: false,
        };
        req.send(&engine_unprivileged_state, move |resp| {
            tx.send(resp.opened_process_info.is_some()).ok();
        });
        assert!(rx.recv().unwrap_or(false), "failed to open pid={}", pid);
    }

    // Build snapshot regions.
    {
        let (tx, rx) = mpsc::channel();
        let req = ScanNewRequest {};
        req.send(&engine_unprivileged_state, move |_resp| {
            tx.send(()).ok();
        });
        let _ = rx.recv();
    }

    // Inspect snapshot.
    let (region_count, byte_count) = {
        let snapshot = engine_privileged_state.get_snapshot();
        let guard = snapshot.read().expect("snapshot read lock");
        (guard.get_region_count(), guard.get_byte_count())
    };
    println!("snapshot regions={} bytes={}", region_count, byte_count);

    // Wait for scan results updates (ScanNew + ElementScan completion both emit this event).
    let (updated_tx, updated_rx) = mpsc::channel();
    engine_unprivileged_state.listen_for_engine_event::<ScanResultsUpdatedEvent>(move |event| {
        // We only care about the scan completing and producing results (not the baseline ScanNew).
        if !event.is_new_scan {
            updated_tx.send(()).ok();
        }
    });

    // Run a trivial scan (4-byte == 0) to validate end-to-end.
    let constraint = AnonymousScanConstraint::new(
        ScanCompareType::Immediate(ScanCompareTypeImmediate::Equal),
        Some(AnonymousValueString::new(
            "0".to_string(),
            AnonymousValueStringFormat::Decimal,
            ContainerType::None,
        )),
    );

    let (tx, rx) = mpsc::channel();
    let req = ElementScanRequest {
        scan_constraints: vec![constraint],
        data_type_refs: vec![DataTypeRef::new(DataTypeI32::get_data_type_id())],
    };
    req.send(&engine_unprivileged_state, move |resp| {
        tx.send(resp.trackable_task_handle.is_some()).ok();
    });
    assert!(rx.recv().unwrap_or(false), "element scan failed");

    // Wait for the engine to publish scan-results-updated.
    updated_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("timed out waiting for ScanResultsUpdatedEvent");

    // Query first page and print counts.
    let (tx, rx) = mpsc::channel();
    let req = ScanResultsQueryRequest {
        page_index: 0,
        page_size: None,
    };
    req.send(&engine_unprivileged_state, move |resp| {
        tx.send(resp).ok();
    });
    let resp = rx.recv_timeout(Duration::from_secs(10)).expect("timed out waiting for ScanResultsQueryResponse");
    println!(
        "results count={} last_page_index={} page_len={} total_bytes={}",
        resp.result_count,
        resp.last_page_index,
        resp.scan_results.len(),
        resp.total_size_in_bytes
    );

    for (idx, r) in resp.scan_results.iter().take(5).enumerate() {
        let value = r
            .get_current_display_value(AnonymousValueStringFormat::Decimal)
            .map(|v| v.get_anonymous_value_string())
            .unwrap_or("??");
        println!("#{idx}: addr=0x{:X} value={value}", r.get_address());
    }
}
