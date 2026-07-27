//! Minimal WinRT RFCOMM lab (gekoppelt → StreamSocket).
//!
//!   cargo run --bin bt_winrt_simple --features rfcomm
//!
//! Kein Winsock AF_BTH: BluetoothDevice + RfcommDeviceService + StreamSocket.
//! Beobachtung: kommt ein Swift-/Pair-Toast? Wird der Bond still genutzt?

use reddot_desktop_lib::rfcomm::{
    discovery::{bond_state, find_reddot_candidate},
    target::RfcommTarget,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn load_known() -> Option<RfcommTarget> {
    let path = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("de.disag.rotpunktarena")
        .join("rfcomm_known_target.json");
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn print_bond(label: &str, addr: u64) {
    match bond_state(addr) {
        Ok(Some(b)) => eprintln!(
            "  bond {label}: auth={} remembered={} connected={}",
            b.authenticated, b.remembered, b.connected
        ),
        Ok(None) => eprintln!("  bond {label}: (unknown)"),
        Err(e) => eprintln!("  bond {label}: err {e}"),
    }
}

#[cfg(windows)]
fn winrt_connect(addr: u64) -> Result<(), String> {
    use windows::Devices::Bluetooth::Rfcomm::{RfcommDeviceService, RfcommServiceId};
    use windows::Devices::Bluetooth::{
        BluetoothCacheMode, BluetoothConnectionStatus, BluetoothDevice, BluetoothError,
    };
    use windows::Devices::Enumeration::DeviceAccessStatus;
    use windows::Networking::Sockets::StreamSocket;

    eprintln!("  FromBluetoothAddressAsync…");
    let t0 = Instant::now();
    let device = BluetoothDevice::FromBluetoothAddressAsync(addr)
        .map_err(|e| format!("FromBluetoothAddressAsync: {e}"))?
        .get()
        .map_err(|e| format!("FromBluetoothAddress get: {e}"))?;
    eprintln!(
        "  device ok ({:.0?}) name={:?} status={:?}",
        t0.elapsed(),
        device.Name().ok().map(|h| h.to_string()),
        device.ConnectionStatus().ok()
    );
    if device.ConnectionStatus().ok() == Some(BluetoothConnectionStatus::Connected) {
        eprintln!("  (WinRT sagt bereits Connected)");
    }

    eprintln!("  RequestAccessAsync…");
    let access = device
        .RequestAccessAsync()
        .map_err(|e| format!("RequestAccessAsync: {e}"))?
        .get()
        .map_err(|e| format!("RequestAccess get: {e}"))?;
    eprintln!("  access={access:?}");
    if access != DeviceAccessStatus::Allowed {
        return Err(format!("DeviceAccess nicht Allowed: {access:?}"));
    }

    let spp = RfcommServiceId::SerialPort().map_err(|e| format!("SerialPort id: {e}"))?;
    eprintln!("  GetRfcommServicesForIdAsync(SPP, Cached)…");
    let t1 = Instant::now();
    let mut services_result = device
        .GetRfcommServicesForIdWithCacheModeAsync(&spp, BluetoothCacheMode::Cached)
        .map_err(|e| format!("GetRfcommServices cached: {e}"))?
        .get()
        .map_err(|e| format!("GetRfcommServices cached get: {e}"))?;
    let mut err = services_result.Error().unwrap_or(BluetoothError::Success);
    let mut n = services_result
        .Services()
        .ok()
        .and_then(|v| v.Size().ok())
        .unwrap_or(0);
    eprintln!("  cached: error={err:?} services={n} ({:.0?})", t1.elapsed());

    if n == 0 || err != BluetoothError::Success {
        eprintln!("  retry GetRfcommServicesForIdAsync(SPP, Uncached)…");
        let t2 = Instant::now();
        services_result = device
            .GetRfcommServicesForIdWithCacheModeAsync(&spp, BluetoothCacheMode::Uncached)
            .map_err(|e| format!("GetRfcommServices uncached: {e}"))?
            .get()
            .map_err(|e| format!("GetRfcommServices uncached get: {e}"))?;
        err = services_result.Error().unwrap_or(BluetoothError::Success);
        n = services_result
            .Services()
            .ok()
            .and_then(|v| v.Size().ok())
            .unwrap_or(0);
        eprintln!("  uncached: error={err:?} services={n} ({:.0?})", t2.elapsed());
    }

    if err != BluetoothError::Success {
        return Err(format!("RfcommServices Error={err:?}"));
    }
    if n == 0 {
        return Err("keine SPP RfcommDeviceService".into());
    }

    let services = services_result
        .Services()
        .map_err(|e| format!("Services(): {e}"))?;
    let service: RfcommDeviceService = services
        .GetAt(0)
        .map_err(|e| format!("Services.GetAt(0): {e}"))?;

    let host = service
        .ConnectionHostName()
        .map_err(|e| format!("ConnectionHostName: {e}"))?;
    let svc_name = service
        .ConnectionServiceName()
        .map_err(|e| format!("ConnectionServiceName: {e}"))?;
    eprintln!(
        "  service host={} name={}",
        host.ToString().unwrap_or_default(),
        svc_name
    );

    eprintln!("  StreamSocket.ConnectAsync… (Toast beobachten)");
    let t3 = Instant::now();
    let socket = StreamSocket::new().map_err(|e| format!("StreamSocket::new: {e}"))?;
    socket
        .ConnectAsync(&host, &svc_name)
        .map_err(|e| format!("ConnectAsync: {e}"))?
        .get()
        .map_err(|e| format!("ConnectAsync get: {e}"))?;
    eprintln!("  Connect OK ({:.0?})", t3.elapsed());

    eprintln!("  holding 3s…");
    std::thread::sleep(Duration::from_secs(3));
    drop(socket);
    let _ = service.Close();
    Ok(())
}

fn main() {
    eprintln!("=== bt_winrt_simple (WinRT Rfcomm + StreamSocket) ===");

    let target = load_known().unwrap_or_else(|| {
        find_reddot_candidate()
            .ok()
            .flatten()
            .expect("Kein Known-Target / Candidate — erst Setup/Pair")
    });
    eprintln!(
        "Target {} @ {:012X}",
        target.display_name, target.bt_addr
    );
    print_bond("before", target.bt_addr);

    match bond_state(target.bt_addr) {
        Ok(Some(b)) if b.authenticated => {}
        Ok(Some(_)) => {
            eprintln!("FAIL: nicht authenticated — erst koppeln");
            std::process::exit(4);
        }
        Ok(None) => {
            eprintln!("FAIL: Gerät unbekannt");
            std::process::exit(4);
        }
        Err(e) => {
            eprintln!("FAIL bond_state: {e}");
            std::process::exit(4);
        }
    }

    #[cfg(windows)]
    {
        let wall = Instant::now();
        match winrt_connect(target.bt_addr) {
            Ok(()) => {
                print_bond("after", target.bt_addr);
                eprintln!("PASS WinRT link wall={:.0?}", wall.elapsed());
            }
            Err(e) => {
                print_bond("after fail", target.bt_addr);
                eprintln!("FAIL {e} wall={:.0?}", wall.elapsed());
                std::process::exit(2);
            }
        }
    }

    #[cfg(not(windows))]
    {
        eprintln!("FAIL: nur Windows");
        std::process::exit(1);
    }
}
