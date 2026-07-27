//! Diagnose-only lab — **not** the product Owner/startup path.
//!
//! Compares Start-path Pair variants (A / Nuclear Light / Full Nuclear) without
//! changing `ConnectionManager`. Product start remains Startup Nuclear (`5b1dcd3`).
//!
//! Hypothesis: L1 with an existing bond is often a Pair no-op
//! (`alreadyAuthenticated`) → effectively Connect A + unused hook.
//!
//!   cargo run --bin bt_start_pair_variants --features rfcomm
//!   cargo run --bin bt_start_pair_variants --features rfcomm -- A
//!   cargo run --bin bt_start_pair_variants --features rfcomm -- L1
//!   cargo run --bin bt_start_pair_variants --features rfcomm -- L2
//!   cargo run --bin bt_start_pair_variants --features rfcomm -- N
//!   cargo run --bin bt_start_pair_variants --features rfcomm -- all
//!
//! JSONL: logs/start_pair_variants.jsonl
//! Soft-Toast: confirm manually (visibleToastObserved).

use reddot_desktop_lib::connection::forget_reddot_bonds;
use reddot_desktop_lib::rfcomm::{
    auth_hook,
    discovery::{
        bond_state, find_reddot_candidate, pair_with_pin_report, BondState, PairApiReport,
        REDDOT_PAIR_PIN,
    },
    spp_com, target::RfcommTarget, RfcommSocket, WinsockRuntime,
};
use serde::Serialize;
use std::fs::{create_dir_all, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    A,
    L1,
    L2,
    N,
}

impl Variant {
    fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::L1 => "L1",
            Self::L2 => "L2",
            Self::N => "N",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "A" => Some(Self::A),
            "L1" => Some(Self::L1),
            "L2" => Some(Self::L2),
            "N" => Some(Self::N),
            _ => None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunLog {
    ts: String,
    run_id: String,
    variant: String,
    initial_bond: String,
    initial_connected: Option<bool>,
    hook_installed: bool,
    pair_api_result: String,
    auth_callback_count: u32,
    pin_response_result: Option<String>,
    release_attempted: bool,
    rfcomm_result: String,
    linked: bool,
    duration_ms: u128,
    visible_toast_observed: bool,
    light_useful: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    forget_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pair_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bond_before_pair: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bond_after_pair: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hook_uninstalled: Option<bool>,
}

fn load_known() -> Option<RfcommTarget> {
    let path = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("de.disag.rotpunktarena")
        .join("rfcomm_known_target.json");
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn resolve() -> Option<RfcommTarget> {
    load_known()
        .or_else(|| find_reddot_candidate().ok().flatten())
        .map(|mut t| {
            t.rfcomm_channel = Some(1);
            t
        })
}

fn wait_enter(prompt: &str) {
    eprint!("{prompt}");
    let _ = io::stderr().flush();
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
}

fn ask_toast() -> bool {
    eprint!("  Soft-Toast sichtbar? [j/N] ");
    let _ = io::stderr().flush();
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
    matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "j" | "y" | "ja" | "yes"
    )
}

fn drain_auth_notes() -> (u32, Option<String>) {
    let mut n = 0u32;
    let mut last = None;
    while let Some(msg) = auth_hook::take_last_auth_note() {
        n += 1;
        last = Some(msg);
    }
    (n, last)
}

fn bond_label(b: Option<BondState>) -> (&'static str, Option<bool>) {
    match b {
        Some(s) if s.authenticated => ("bonded", Some(s.connected)),
        Some(_) => ("notBonded", Some(false)),
        None => ("unknown", None),
    }
}

fn pair_result_str(r: &PairApiReport) -> String {
    match r {
        PairApiReport::Success { win32 } => format!("success(win32={win32})"),
        PairApiReport::AlreadyAuthenticated { reason, win32 } => match win32 {
            Some(c) => format!("alreadyAuthenticated({reason},win32={c})"),
            None => format!("alreadyAuthenticated({reason})"),
        },
        PairApiReport::Error { win32, message } => match win32 {
            Some(c) => format!("error(win32={c}:{message})"),
            None => format!("error({message})"),
        },
    }
}

fn log_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../logs")
        .join("start_pair_variants.jsonl")
}

fn bond_snapshot(addr: u64) -> String {
    match bond_state(addr) {
        Ok(Some(b)) => format!(
            "auth={} rem={} connected={}",
            b.authenticated, b.remembered, b.connected
        ),
        Ok(None) => "none".into(),
        Err(e) => format!("err:{e}"),
    }
}

fn append_log(row: &RunLog) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = create_dir_all(parent);
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        if let Ok(line) = serde_json::to_string(row) {
            let _ = writeln!(f, "{line}");
        }
    }
    if let Ok(line) = serde_json::to_string_pretty(row) {
        eprintln!("\n--- JSONL ---\n{line}\n-------------");
    }
}

fn rfcomm_once(target: &RfcommTarget) -> Result<(), String> {
    let page = Duration::from_secs(12);
    eprint!("  RFCOMM connect ({page:?})… ");
    let t0 = Instant::now();
    match RfcommSocket::connect(target, page) {
        Ok(sock) => {
            eprintln!("OK ({:.0?})", t0.elapsed());
            std::thread::sleep(Duration::from_secs(2));
            drop(sock);
            spp_com::restore_all();
            Ok(())
        }
        Err(e) => {
            eprintln!("FAIL ({:.0?}) — {e}", t0.elapsed());
            Err(e.to_string())
        }
    }
}

fn run_variant(variant: Variant, target: &RfcommTarget, run_id: &str) {
    eprintln!("\n======== VARIANT {} ========", variant.as_str());
    match variant {
        Variant::A => eprintln!("  Bonded → RFCOMM (Baseline)"),
        Variant::L1 => eprintln!("  Hook → pair_with_pin → RFCOMM (Nuclear Light, kein Release)"),
        Variant::L2 => eprintln!("  Hook → pair → Release → RFCOMM"),
        Variant::N => eprintln!("  Forget → Hook → Pair → RFCOMM (Full Nuclear)"),
    }

    let bond0 = bond_state(target.bt_addr).ok().flatten();
    let (initial_bond, initial_connected) = bond_label(bond0);
    eprintln!(
        "  initialBond={initial_bond} connected={initial_connected:?}"
    );
    wait_enter("  Enter = Lauf starten (Soft-Toast beobachten)… ");

    let wall = Instant::now();
    let mut hook_installed = false;
    let mut pair_api_result = "n/a".to_string();
    let mut auth_callback_count = 0u32;
    let mut pin_response_result = None;
    let mut release_attempted = false;
    let rfcomm_result;
    let mut linked = false;
    let mut light_useful = None;
    let mut forget_ms = None;
    let mut pair_ms = None;
    let mut bond_before_pair = None;
    let mut bond_after_pair = None;
    let mut hook_uninstalled = None;

    let _ = drain_auth_notes();

    match variant {
        Variant::A => {
            match rfcomm_once(target) {
                Ok(()) => {
                    rfcomm_result = "ok".into();
                    linked = true;
                }
                Err(e) => rfcomm_result = format!("fail:{e}"),
            }
        }
        Variant::L1 | Variant::L2 => {
            if let Err(e) = auth_hook::install_reddot_pin_hook() {
                rfcomm_result = format!("hook_fail:{e}");
                let toast = ask_toast();
                append_log(&RunLog {
                    ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    run_id: run_id.into(),
                    variant: variant.as_str().into(),
                    initial_bond: initial_bond.into(),
                    initial_connected,
                    hook_installed: false,
                    pair_api_result: "n/a".into(),
                    auth_callback_count: 0,
                    pin_response_result: None,
                    release_attempted: false,
                    rfcomm_result,
                    linked: false,
                    duration_ms: wall.elapsed().as_millis(),
                    visible_toast_observed: toast,
                    light_useful: Some(false),
                    forget_ms: None,
                    pair_ms: None,
                    bond_before_pair: None,
                    bond_after_pair: None,
                    hook_uninstalled: Some(true),
                });
                auth_hook::clear_auto_pin_allows();
                auth_hook::uninstall_reddot_pin_hook();
                return;
            }
            auth_hook::allow_auto_pin_for(target.bt_addr);
            hook_installed = true;

            let report = pair_with_pin_report(
                target.bt_addr,
                &target.display_name,
                REDDOT_PAIR_PIN,
            );
            pair_api_result = pair_result_str(&report);
            let (n, last) = drain_auth_notes();
            auth_callback_count = n;
            pin_response_result = last;
            eprintln!("  pairApiResult={pair_api_result}");
            eprintln!("  authCallbackCount={auth_callback_count}");

            light_useful = Some(matches!(
                report,
                PairApiReport::Success { .. }
            ) && auth_callback_count >= 1);

            if variant == Variant::L2 {
                release_attempted = true;
                match spp_com::release_channel_for(target.bt_addr) {
                    spp_com::SppComAction::None => eprintln!("  spp_com: none"),
                    spp_com::SppComAction::Disabled { com, .. } => {
                        eprintln!("  spp_com: disabled {com}")
                    }
                    spp_com::SppComAction::Busy { com, detail } => {
                        eprintln!("  spp_com: busy {com} ({detail})")
                    }
                }
                std::thread::sleep(Duration::from_secs(2));
            }

            match rfcomm_once(target) {
                Ok(()) => {
                    rfcomm_result = "ok".into();
                    linked = true;
                }
                Err(e) => rfcomm_result = format!("fail:{e}"),
            }

            auth_hook::clear_auto_pin_allows();
            auth_hook::uninstall_reddot_pin_hook();
            hook_uninstalled = Some(true);
            spp_com::restore_all();
        }
        Variant::N => {
            // Instrumentiert (nicht opaque run_nuclear_link) — Vorbehalt authCallbackCount.
            eprintln!("  [N1] Forget…");
            let t_forget = Instant::now();
            forget_reddot_bonds(target.bt_addr);
            std::thread::sleep(Duration::from_secs(2));
            forget_ms = Some(t_forget.elapsed().as_millis());
            eprintln!("  forgetMs={}", forget_ms.unwrap());

            bond_before_pair = Some(bond_snapshot(target.bt_addr));
            eprintln!("  bondBeforePair={}", bond_before_pair.as_ref().unwrap());

            eprintln!("  [N2] Hook + pair_with_pin_report…");
            if let Err(e) = auth_hook::install_reddot_pin_hook() {
                rfcomm_result = format!("hook_fail:{e}");
            } else {
                auth_hook::allow_auto_pin_for(target.bt_addr);
                hook_installed = true;
                let t_pair = Instant::now();
                let mut report = pair_with_pin_report(
                    target.bt_addr,
                    &target.display_name,
                    REDDOT_PAIR_PIN,
                );
                // After Forget, precheck should not short-circuit; retry once if needed.
                if matches!(report, PairApiReport::AlreadyAuthenticated { .. }) {
                    eprintln!("  pair unexpected alreadyAuth after Forget — retry once");
                    std::thread::sleep(Duration::from_secs(1));
                    report = pair_with_pin_report(
                        target.bt_addr,
                        &target.display_name,
                        REDDOT_PAIR_PIN,
                    );
                }
                pair_ms = Some(t_pair.elapsed().as_millis());
                pair_api_result = pair_result_str(&report);
                let (n, last) = drain_auth_notes();
                auth_callback_count = n;
                pin_response_result = last;
                bond_after_pair = Some(bond_snapshot(target.bt_addr));
                eprintln!("  pairApiResult={pair_api_result} pairMs={}", pair_ms.unwrap());
                eprintln!("  authCallbackCount={auth_callback_count}");
                eprintln!("  bondAfterPair={}", bond_after_pair.as_ref().unwrap());

                auth_hook::clear_auto_pin_allows();
                auth_hook::uninstall_reddot_pin_hook();

                // Product Nuclear also releases before RFCOMM — keep for parity; toast noted separately.
                release_attempted = true;
                let _ = spp_com::release_channel_for(target.bt_addr);
                std::thread::sleep(Duration::from_secs(1));

                eprintln!("  [N3] Hook + RFCOMM…");
                if let Err(e) = auth_hook::install_reddot_pin_hook() {
                    rfcomm_result = format!("hook2_fail:{e}");
                } else {
                    auth_hook::allow_auto_pin_for(target.bt_addr);
                    match rfcomm_once(target) {
                        Ok(()) => {
                            let (n2, last2) = drain_auth_notes();
                            auth_callback_count += n2;
                            if last2.is_some() {
                                pin_response_result = last2;
                            }
                            rfcomm_result = "ok".into();
                            linked = true;
                        }
                        Err(e) => rfcomm_result = format!("fail:{e}"),
                    }
                    auth_hook::clear_auto_pin_allows();
                    auth_hook::uninstall_reddot_pin_hook();
                }
            }
            hook_uninstalled = Some(true);
            spp_com::restore_all();
        }
    }

    let toast = ask_toast();
    let duration_ms = wall.elapsed().as_millis();

    // Decisive metric for L1/L2
    if matches!(variant, Variant::L1 | Variant::L2) {
        let useful = pair_api_result.starts_with("success")
            && auth_callback_count >= 1
            && !toast
            && linked;
        light_useful = Some(useful);
        eprintln!(
            "  decisive: pair!=alreadyAuth & authCb>=1 & !toast & linked → {useful}"
        );
    }

    append_log(&RunLog {
        ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        run_id: run_id.into(),
        variant: variant.as_str().into(),
        initial_bond: initial_bond.into(),
        initial_connected,
        hook_installed,
        pair_api_result,
        auth_callback_count,
        pin_response_result,
        release_attempted,
        rfcomm_result,
        linked,
        duration_ms,
        visible_toast_observed: toast,
        light_useful,
        forget_ms,
        pair_ms,
        bond_before_pair,
        bond_after_pair,
        hook_uninstalled,
    });
}

fn main() {
    eprintln!("=== bt_start_pair_variants (DIAGNOSE-ONLY — not product path) ===");
    eprintln!("A=RFCOMM | L1=Light | L2=Light+Release | N=Full Nuclear");
    eprintln!("Product start = Startup Nuclear; this bin does not change Owner.");
    eprintln!("JSONL → {}", log_path().display());

    if let Err(e) = WinsockRuntime::init() {
        eprintln!("WSAStartup failed: {e}");
        std::process::exit(1);
    }

    let Some(target) = resolve() else {
        eprintln!("FAIL kein Known/Candidate");
        std::process::exit(4);
    };
    eprintln!(
        "Target {} @ {:012X}",
        target.display_name, target.bt_addr
    );

    let arg = std::env::args().nth(1).unwrap_or_else(|| "all".into());
    let variants: Vec<Variant> = if arg.eq_ignore_ascii_case("all") {
        vec![Variant::A, Variant::L1, Variant::L2, Variant::N]
    } else if let Some(v) = Variant::parse(&arg) {
        vec![v]
    } else {
        eprintln!("FAIL unknown variant {arg:?} — use A|L1|L2|N|all");
        std::process::exit(1);
    };

    let run_id = format!(
        "r{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%S")
    );
    eprintln!("runId={run_id}");

    for (i, v) in variants.iter().enumerate() {
        if i > 0 {
            eprintln!("\n  settle 8s zwischen Varianten…");
            // After N, bond is fresh — for fair L* vs A on bonded idle, run A/L first.
            std::thread::sleep(Duration::from_secs(8));
        }
        // N forgets — warn if running all (order A,L1,L2,N is intentional)
        if *v == Variant::N {
            eprintln!("  Hinweis: N macht Forget — Bond danach neu.");
        }
        run_variant(*v, &target, &run_id);
    }

    eprintln!("\n======== DONE ========");
    eprintln!("Messwert Light: pairApiResult=success AND authCallbackCount>=1 AND !toast AND linked");
    eprintln!("Wenn L1 oft alreadyAuthenticated → Light == A, nicht Produkt-Start.");
}
