//! RedDot serial protocol (mirror of packages/protocol, docs/protocol.md).

pub const STX: u8 = 0x02;
pub const ENQ: u8 = 0x05;
pub const ACK: u8 = 0x06;
pub const DC1: u8 = 0x11;
pub const NAK: u8 = 0x15;

pub const SHOT_FRAME_LENGTH: usize = 59;
#[allow(dead_code)]
pub const BAUD_RATE: u32 = 9600;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shot {
    pub value_raw: i32,
    pub distance_raw: i32,
    pub x: i32,
    pub y: i32,
    pub value_display: f64,
    pub distance_display: f64,
}

#[derive(Debug, Clone)]
pub enum Incoming {
    Nak,
    Ack,
    /// Complete 59-byte STX frame (raw bytes preserved for integrity pipeline).
    ShotFrame(Vec<u8>),
    NeedMore,
    Skip,
}

fn ascii_field(buf: &[u8], offset: usize, length: usize) -> String {
    buf[offset..offset + length]
        .iter()
        .map(|&b| b as char)
        .collect()
}

fn parse_dotted_int(field: &str) -> Result<i32, String> {
    let cleaned: String = field.chars().filter(|c| *c != '.').collect();
    cleaned
        .parse::<i32>()
        .map_err(|e| format!("parse int '{field}': {e}"))
}

pub fn parse_shot_frame(frame: &[u8]) -> Result<Shot, String> {
    if frame.len() < SHOT_FRAME_LENGTH {
        return Err(format!("Shot frame too short: {}", frame.len()));
    }
    if frame[0] != STX {
        return Err(format!("Expected STX, got 0x{:02x}", frame[0]));
    }

    let value_raw = parse_dotted_int(&ascii_field(frame, 32, 4))?;
    let distance_raw = parse_dotted_int(&ascii_field(frame, 37, 6))?;
    let x = ascii_field(frame, 44, 5)
        .parse::<i32>()
        .map_err(|e| format!("x: {e}"))?;
    let y = ascii_field(frame, 50, 5)
        .parse::<i32>()
        .map_err(|e| format!("y: {e}"))?;

    Ok(Shot {
        value_raw,
        distance_raw,
        x,
        y,
        value_display: value_raw as f64 / 10.0,
        distance_display: distance_raw as f64 / 10.0,
    })
}

/// Incremental byte-stream consumer. Emits raw frames; parsing is Arena Core's job.
#[derive(Default)]
pub struct RedDotStreamParser {
    buffer: Vec<u8>,
}

impl RedDotStreamParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inspect buffer without consuming (tests / incomplete check).
    pub fn buffer_bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// True only if the parser currently retains a valid, incomplete
    /// STX-led shot-frame candidate **after** its normal feed/resync work
    /// (`push` → `drain`: NAK/ACK consumed, non-framed prefix bytes skipped).
    ///
    /// - empty buffer → false
    /// - full frame already drained → false
    /// - invalid prefix already resynced away → false
    /// - leading STX + fewer than [`SHOT_FRAME_LENGTH`] bytes → true
    pub fn has_incomplete_shot_frame(&self) -> bool {
        let buf = self.buffer_bytes();
        !buf.is_empty() && buf[0] == STX && buf.len() < SHOT_FRAME_LENGTH
    }

    pub fn push(&mut self, bytes: &[u8]) -> Vec<Incoming> {
        self.buffer.extend_from_slice(bytes);
        self.drain()
    }

    fn drain(&mut self) -> Vec<Incoming> {
        let mut out = Vec::new();
        loop {
            if self.buffer.is_empty() {
                break;
            }
            let b = self.buffer[0];
            match b {
                NAK => {
                    self.buffer.remove(0);
                    out.push(Incoming::Nak);
                }
                ACK => {
                    self.buffer.remove(0);
                    out.push(Incoming::Ack);
                }
                STX => {
                    if self.buffer.len() < SHOT_FRAME_LENGTH {
                        out.push(Incoming::NeedMore);
                        break;
                    }
                    let frame: Vec<u8> = self.buffer.drain(..SHOT_FRAME_LENGTH).collect();
                    out.push(Incoming::ShotFrame(frame));
                }
                _ => {
                    self.buffer.remove(0);
                    out.push(Incoming::Skip);
                }
            }
        }
        out
    }
}

/// Synthetic 59-byte STX frame (unknown regions filled with spaces).
pub fn build_synthetic_shot_frame(
    value_ascii: &str,
    distance_ascii: &str,
    x_ascii: &str,
    y_ascii: &str,
) -> Result<Vec<u8>, String> {
    if value_ascii.len() != 4 {
        return Err("valueAscii must be length 4".into());
    }
    if distance_ascii.len() != 6 {
        return Err("distanceAscii must be length 6".into());
    }
    if x_ascii.len() != 5 || y_ascii.len() != 5 {
        return Err("x/y Ascii must be length 5".into());
    }
    let mut frame = vec![0x20u8; SHOT_FRAME_LENGTH];
    frame[0] = STX;
    frame[32..36].copy_from_slice(value_ascii.as_bytes());
    frame[37..43].copy_from_slice(distance_ascii.as_bytes());
    frame[44..49].copy_from_slice(x_ascii.as_bytes());
    frame[50..55].copy_from_slice(y_ascii.as_bytes());
    Ok(frame)
}

/// Stamp unused header bytes so identical aims still produce unique SHA-256 (click-to-shoot).
pub fn stamp_frame_nonce(frame: &mut [u8]) {
    if frame.len() < 17 {
        return;
    }
    let n = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or(0)
        .unsigned_abs();
    let stamp = format!("{n:016}");
    let bytes = stamp.as_bytes();
    let start = bytes.len().saturating_sub(16);
    frame[1..17].copy_from_slice(&bytes[start..]);
}

/// Simulator / mouse-aim outer 1-ring radius — matches TargetFace `DEVICE_RADIUS_AT_RING_1`.
/// Hardware frames are not produced here; do not change ingest mapping.
pub const AIM_RADIUS_AT_RING_1: f64 = 2500.0;

/// Format distance ASCII so `distance_display` (== raw/10) ≈ hypot(x,y).
fn format_aim_distance_ascii(r: f64) -> String {
    let d = r.clamp(0.0, 9999.9);
    let s = format!("{d:06.1}");
    if s.len() == 6 {
        s
    } else if s.len() > 6 {
        s.chars().take(6).collect()
    } else {
        format!("{s:0>6}")
    }
}

/// Map aim coordinates → protocol ASCII fields (simulator / mouse-aim only).
pub fn aim_coords_to_ascii(x: f64, y: f64) -> (String, String, String, String) {
    let xi = (x.round() as i32).clamp(-9999, 9999);
    let yi = (y.round() as i32).clamp(-9999, 9999);
    let r = ((xi * xi + yi * yi) as f64).sqrt();
    // Same linear shape as the old 450 scale, stretched to AIM_RADIUS_AT_RING_1.
    let tenths =
        ((109.0 - (r / AIM_RADIUS_AT_RING_1) * 100.0).round() as i32).clamp(0, 109);
    let value = {
        let s = format!("{:.1}", tenths as f64 / 10.0);
        if s.len() >= 4 {
            s.chars().take(4).collect()
        } else {
            format!("{s:0>4}")
        }
    };
    let dist = format_aim_distance_ascii(r);
    let x_ascii = if xi < 0 {
        format!("-{:04}", xi.unsigned_abs())
    } else {
        format!("{xi:05}")
    };
    let y_ascii = if yi < 0 {
        format!("-{:04}", yi.unsigned_abs())
    } else {
        format!("{yi:05}")
    };
    (value, dist, x_ascii, y_ascii)
}

pub fn encode_enq() -> Vec<u8> {
    vec![ENQ]
}

pub fn encode_ack() -> Vec<u8> {
    vec![ACK]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_shot_frame_state_after_feed() {
        let frame = build_synthetic_shot_frame("09.5", "000.10", "00000", "00000").unwrap();
        let mut p = RedDotStreamParser::new();
        assert!(!p.has_incomplete_shot_frame());

        // Noise resynced away → not incomplete.
        let _ = p.push(&[0x00, 0xFF, NAK]);
        assert!(!p.has_incomplete_shot_frame());

        // Partial STX candidate → true.
        let mid = 20;
        let ev1 = p.push(&frame[..mid]);
        assert!(matches!(ev1[..], [Incoming::NeedMore]));
        assert!(p.has_incomplete_shot_frame());

        // Completing chunk → false.
        let ev2 = p.push(&frame[mid..]);
        assert!(matches!(ev2[..], [Incoming::ShotFrame(_)]));
        assert!(!p.has_incomplete_shot_frame());
    }

    #[test]
    fn enq_nak_ack_bytes() {
        assert_eq!(encode_enq(), vec![ENQ]);
        assert_eq!(encode_ack(), vec![ACK]);
        let mut p = RedDotStreamParser::new();
        assert!(matches!(p.push(&[NAK])[..], [Incoming::Nak]));
        assert!(matches!(p.push(&[ACK])[..], [Incoming::Ack]));
    }

    #[test]
    fn stx_frame_complete() {
        let frame = build_synthetic_shot_frame("10.9", "012.34", "00100", "-0050").unwrap();
        assert_eq!(frame.len(), SHOT_FRAME_LENGTH);
        assert_eq!(frame[0], STX);
        let mut p = RedDotStreamParser::new();
        let ev = p.push(&frame);
        assert_eq!(ev.len(), 1);
        match &ev[0] {
            Incoming::ShotFrame(f) => {
                let shot = parse_shot_frame(f).unwrap();
                assert_eq!(shot.value_raw, 109);
                assert_eq!(shot.x, 100);
                assert_eq!(shot.y, -50);
            }
            other => panic!("expected ShotFrame, got {other:?}"),
        }
    }

    #[test]
    fn stx_fragment_then_complete() {
        let frame = build_synthetic_shot_frame("09.5", "000.10", "00000", "00000").unwrap();
        let mut p = RedDotStreamParser::new();
        let mid = 20;
        let ev1 = p.push(&frame[..mid]);
        assert!(matches!(ev1[..], [Incoming::NeedMore]));
        let ev2 = p.push(&frame[mid..]);
        assert!(matches!(ev2[..], [Incoming::ShotFrame(_)]));
    }

    #[test]
    fn skip_noise_before_nak() {
        let mut p = RedDotStreamParser::new();
        let ev = p.push(&[0x00, 0xFF, NAK]);
        assert!(matches!(
            ev[..],
            [Incoming::Skip, Incoming::Skip, Incoming::Nak]
        ));
    }

    #[test]
    fn multiple_frames_in_one_push() {
        let a = build_synthetic_shot_frame("10.0", "001.00", "00001", "00002").unwrap();
        let b = build_synthetic_shot_frame("10.1", "002.00", "00003", "00004").unwrap();
        let mut buf = Vec::new();
        buf.extend_from_slice(&a);
        buf.push(NAK);
        buf.extend_from_slice(&b);
        let mut p = RedDotStreamParser::new();
        let ev = p.push(&buf);
        assert_eq!(ev.len(), 3);
        assert!(matches!(ev[0], Incoming::ShotFrame(_)));
        assert!(matches!(ev[1], Incoming::Nak));
        assert!(matches!(ev[2], Incoming::ShotFrame(_)));
    }

    #[test]
    fn aim_coords_outer_ring_uses_device_radius() {
        let (value, dist, x, y) = aim_coords_to_ascii(AIM_RADIUS_AT_RING_1, 0.0);
        assert_eq!(x, "02500");
        assert_eq!(y, "00000");
        assert_eq!(value, "00.9");
        assert_eq!(dist, "2500.0");
        let frame = build_synthetic_shot_frame(&value, &dist, &x, &y).unwrap();
        let shot = parse_shot_frame(&frame).unwrap();
        assert_eq!(shot.x, 2500);
        assert_eq!(shot.y, 0);
        assert!((shot.distance_display - AIM_RADIUS_AT_RING_1).abs() < 0.05);
        assert!((shot.value_display - 0.9).abs() < 0.05);
    }

    #[test]
    fn aim_coords_center_is_high_value() {
        let (value, dist, x, y) = aim_coords_to_ascii(0.0, 0.0);
        assert_eq!(x, "00000");
        assert_eq!(y, "00000");
        assert_eq!(value, "10.9");
        assert_eq!(dist, "0000.0");
        let frame = build_synthetic_shot_frame(&value, &dist, &x, &y).unwrap();
        let shot = parse_shot_frame(&frame).unwrap();
        assert!((shot.distance_display - 0.0).abs() < 0.05);
        assert!((shot.value_display - 10.9).abs() < 0.05);
    }
}
