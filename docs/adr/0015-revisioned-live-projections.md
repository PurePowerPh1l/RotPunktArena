# ADR 0015: Revisionierte Live-Projektionen

Status: angenommen, 2026-10-01. Befund F10.

Snapshots enthalten eine pro Prozess monotone Revision, Session-ID und Phase
idle/probe/match/closed. Die Revision wird beim Erzeugen einer Projektion
unter dem Zustandslock zugeteilt; auch reine Reads erhöhen sie. Sie ist keine
persistierte Event-Sequenz. Die UI abonniert vor dem ersten Read den Kanal
live_state und übernimmt nur neuere vollständige Projektionen. Ereignisse
transportieren den gesamten Zustand, sodass Revisionslücken keine fehlerhafte
Delta-Anwendung verursachen. Ein Resync alle fünf Sekunden bei sichtbarer UI
repariert auch ein verlorenes letztes Ereignis. Alte Kanalnamen bleiben für
Diagnose kompatibel; die Produktanzeige verwendet ausschließlich live_state.

Asynchrone Subscription-Handles und Reads sind an eine Effect-Generation
gebunden und werden beim Cleanup verworfen. Command-Antworten durchlaufen
dieselbe Revisionsprüfung. Der RFCOMM-Enumwert wird jetzt auch im TS-Contract
geführt. Gemeinsames idle-JSON prüft Nullwerte, Feldnamen und Rust-Roundtrip.
Tests prüfen verspätete Probe-, Abschluss- und frühere Session-Zustände sowie
vollständigen Ersatz bei einer Lücke.

Vollständige Projektionen kopieren die aktuelle Schussliste. Das ist für
normale Serien eine überschaubare Korrektheitslösung; große Endlossessions
benötigen eine getrennte Messung und gegebenenfalls ein Live-Fenster mit
SQLite-Historienzugriff (P2-Nacharbeit). Keine zusätzlichen Event-Bus- oder
State-Machine-Frameworks werden eingeführt. Interaktive WebView-Abnahme bleibt
zusätzlich zur automatisierten Ordnungsprüfung erforderlich.
