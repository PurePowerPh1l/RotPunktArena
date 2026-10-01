# ADR: Besitzergebundene Session-Sinks

Status: Accepted
Date: 2026-10-01

Die RFCOMM-Bridge registriert eine pro Prozess monoton steigende Lease-ID.
Der Connection-Owner bestätigt die individuelle Registrierung über einen
Antwortkanal mit einer neuen Empfangs-Epoch. Veraltete oder doppelte
Registrierungen werden abgewiesen. Eine Abmeldung entfernt ausschließlich
die aktuelle Lease und bestätigt, ob sie entfernt wurde. Ein verspäteter Close
einer vorherigen Bridge bleibt wirkungslos.

Die Shared-Queue wird beim Registrieren geleert. Eine Bridge darf sie nur lesen,
solange ihre Epoch aktuell ist, damit ein alter Worker keine neuen Frames
entnimmt. Antwort-Timeout: 500 ms. Ein bereits geschlossener Antwortkanal
installiert keinen Sink; bei einem Timeout sendet der Handle eine gezielte
Abmeldung. Fehler bei Open/Close werden an den Transport-Aufrufer gegeben.
Der Socket bleibt Eigentum des appweiten Connection-Owners.

Tests unter `connection::owner::lease_tests` verwenden den echten Command-
Dispatcher und die echten Bridges ohne Bluetooth-FFI: verzögerter alter Close,
alte/doppelte Registrierung, Queue-Schutz, verwaiste Registrierung sowie
bestätigtes Open/Close zweier Sessions. Hardware-Wechsel bleiben manuell abzunehmen.
