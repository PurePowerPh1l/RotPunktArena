# Transport-Adapter (Stand-Client)

Ziel: Das Byte-Protokoll bleibt transportunabhängig. Die App spricht RedDot über einen **Transport-Adapter**; Parser, Event-Log und UI ändern sich nicht, wenn der physische Kanal wechselt.

> **Ausführlich:** Architektur, First-Setup, Bond-Gates, Timeouts, ENQ/Sink und Windows-Eigenheiten stehen in [`bluetooth-connection-stack.md`](./bluetooth-connection-stack.md).

## Architekturentscheidung (Windows Hardware)

**Datenkanal:** nur Winsock RFCOMM (`AF_BTH`, `SOCK_STREAM`, `BTHPROTO_RFCOMM`, `SOCKADDR_BTH`, `WSALookupService*`).

**Nicht verwendet:** Virtual COM / BthModem, WinRT `Windows.Devices.Bluetooth.Rfcomm`, PnP-Recovery, Radio-Toggle, `IOCTL_BTH_DISCONNECT` als Normalpfad.

Begründung: unverpackte Tauri-EXE ohne Package Identity; Winsock braucht keine Manifest-Capabilities und liefert denselben Byte-Stream (ENQ/NAK/STX/ACK).

Identität des Ziels: persistierte **BD_ADDR** (nicht Anzeigename, nicht RFCOMM-Kanalnummer; andere S/N = andere Adresse). SPP-UUID: `00001101-0000-1000-8000-00805F9B34FB`.

**Bond-Gate:** Start + authentifizierter OS-Bond → Soft-Reconnect; sonst Idle. Badge/Setup **Verbinden** = Nuclear (Forget→Pair PIN `0000`→RFCOMM). Kein Soft-Wake-Dauerloop. Kein Known → Setup-Sheet.

## Muster

```text
Ziel (Bluetooth SPP)
    │ RFCOMM bytes
    ▼
Connection Manager (Owner-Thread, Socket)
    │ ByteTransport / Session-Sink
    ▼
Protocol Parser  →  SQLite Event-Log  →  Live-UI
```

Module:

- `transport/rfcomm/` — WinsockRuntime, RfcommSocket, Discovery, Target, Errors
- `connection/` — Handle, Owner, Connect-Cycle, Keepalive, Setup-Flow, Bridge, Policy, Persist
- `protocol` — transportunabhängig

Session-Stop = Sink abmelden; Socket + ENQ bleiben. App-Exit = `shutdown`/`closesocket`, kein Settle/Radio/PnP.

## Gemeinsame Schnittstellen

Session-Bridge (`Transport`):

- `kind()` — `simulator` | `rfcomm` | (legacy `serial`) | `tcp`
- `open()` / `close()` — bei RFCOMM: RegisterSink / UnregisterSink
- `write_all` / `read_timeout`

Niedriger (`ByteTransport`): `read` / `write_all` / `shutdown` mit Timeout.

Reconnect: Produktpfad hat **keinen** Soft-Wake-Dauerloop. Soft nur auf Start bei Bond; sonst Idle → User Nuclear. Single-Flight / Generation bleiben gültig.

## Implementierte Adapter

| Adapter | Status | Feature-Flag |
|---|---|---|
| `SimulatorTransport` | MVP, ohne Hardware | default `simulator` |
| `RfcommSocket` + Connection Manager | Default-Hardwarepfad (Windows) | default `rfcomm` |
| `SerialTransport` (COM) | Legacy, nicht aktiver Pfad | `--features serial` |
| `TcpTransport` | **nicht implementiert** (Phase T) | — |

```bash
cd apps/desktop/src-tauri
cargo build
# Legacy COM nur bei Bedarf:
cargo build --features serial --no-default-features --features simulator,serial
```

## Troubleshooting (ohne COM)

| Symptom | Aktion |
|---|---|
| Idle „tippe Verbinden“ | Soft fail oder kein Bond — Badge **Verbinden** = Nuclear |
| `NeedsTarget` | Kein Known — First-Setup-Sheet |
| `NeedsPairing` | Nuclear/Auth-Fehler — erneut Verbinden |
| `Connecting` | Soft oder Nuclear läuft |
| `Faulted` | Persistenz/Discover — Forget oder Neustart; s. unten |
| Session gestoppt, Link bleibt | Erwartet — ENQ ohne Sink |
| Sticky AccessDenied / COM | Entfällt — kein Virtual COM |

### Status `faulted` — Recovery

Owner startet aus `faulted` **keinen** Connect-Loop.

| Aktion | Ergebnis |
|---|---|
| `rfcomm_forget_target` | → `needsTarget` (Setup) |
| `rfcomm_reconnect` | → Nuclear auf Known |
| Setup Nuclear Erfolg | → `linked` |
| App-Neustart | Soft wenn Bond OK, sonst Idle / NeedsTarget |

## Labs / Soak

Primär: [`rfcomm-nuclear-test-matrix.md`](./rfcomm-nuclear-test-matrix.md).  
Historische Soak-Vorlage: [`rfcomm-soak-baseline.md`](./rfcomm-soak-baseline.md).

## Phase T — Ethernet / TcpTransport (noch nicht)

Unverändert vorgesehen: Bridge spricht RedDot-Serial am Ziel, LAN als `TcpTransport` hinter demselben Trait.
