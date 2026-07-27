# ADR: Native Winsock RFCOMM statt Virtual COM

## Status

Accepted (`feat/rfcomm-bond-gate`; zuvor `feat/native-rfcomm`)

## Kontext

Der frühere Pfad über Windows Virtual COM (BT-SPP / BthModem) führte zu sticky `AccessDenied` nach Soft-Close. Recovery über PnP, Settle-Delays oder Bluetooth-Radio-Toggle war unzuverlässig bzw. produktseitig unerwünscht (jeder Start wirkte wie Radio-Reparatur).

## Entscheidung

Hardware-Datenkanal ausschließlich über **Winsock Bluetooth RFCOMM** (`AF_BTH`). WinRT-RFCOMM, Hybrid-COM und Radio/PnP-Recovery sind ausgeschlossen.

## Konsequenzen

- Keine Package Identity / Manifest-Capabilities nötig für den Datenkanal
- Zielidentität = BD_ADDR in `rfcomm_known_target.json`
- Autoconnect beim App-Start nur bei authentifiziertem Bond (**Soft**); sonst Idle bis User **Nuclear** (Verbinden)
- Nuclear: Forget → Pair (`BluetoothAuthenticateDevice` + AuthEx, PIN `0000`) → RFCOMM; Soft: begrenzte RFCOMM-Pages ohne Forget
- Kein Soft-Wake-Dauerloop; Link lost → Idle
- COM-UI und Repair-Dialoge entfallen
- Legacy-`serial`-Feature bleibt optional, wird auf diesem Branch nicht genutzt
