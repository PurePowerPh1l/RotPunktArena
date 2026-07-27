//! Competition results — best series ranking + all series per entry.

use super::Database;
use rusqlite::params;

/// Shot projection for results UI — same camelCase shape as live `UiShot`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultShot {
    pub shot_index: u32,
    pub value_raw: i32,
    pub distance_raw: i32,
    pub x: i32,
    pub y: i32,
    pub value_display: f64,
    pub distance_display: f64,
    pub series_total: f64,
    pub series_teiler_total: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryResultSummary {
    pub entry_id: String,
    pub competition_id: String,
    pub person_id: String,
    pub start_order: i64,
    pub status: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub club: Option<String>,
    /// Best series session (by scoring mode), if any.
    pub session_id: Option<String>,
    pub session_ended_at: Option<String>,
    pub shot_count: i64,
    pub punkte_total: f64,
    pub teiler_sum: f64,
    pub teiler_avg: f64,
    /// 1-based rank by points (higher better); `None` if no shots.
    pub rank_punkte: Option<i64>,
    /// 1-based rank by teiler avg (lower better); `None` if no shots.
    pub rank_teiler: Option<i64>,
}

/// One series (session) of an entry — chronological index, aggregates, best flag.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesResultSummary {
    pub session_id: String,
    /// 1-based chronological series number (1 = Hauptrunde).
    pub series_index: i64,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub shot_count: i64,
    pub punkte_total: f64,
    pub teiler_sum: f64,
    pub teiler_avg: f64,
    pub is_best: bool,
    /// `true` when `series_index > 1` (Nachkauf series).
    pub is_nachkauf: bool,
    /// Populated by `get_entry_result`; empty for `list_entry_series`.
    #[serde(default)]
    pub shots: Vec<ResultShot>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryResultDetail {
    /// Aggregates from the best series (by competition scoring mode).
    pub summary: EntryResultSummary,
    pub competition_name: String,
    pub scoring_mode: String,
    /// Competition `max_shots` (per series).
    pub max_shots: i64,
    /// Shots of the best series (compat for existing UI).
    pub shots: Vec<ResultShot>,
    /// All series chronologically; best marked with `isBest`.
    pub series: Vec<SeriesResultSummary>,
}

impl Database {
    /// All starters of a competition with aggregates from their **best** series.
    pub fn list_competition_results(
        &self,
        competition_id: &str,
    ) -> Result<Vec<EntryResultSummary>, String> {
        let teiler = self
            .get_competition(competition_id)?
            .map(|c| c.scoring_mode == "teiler")
            .unwrap_or(false);
        let sql = if teiler {
            list_results_sql(BestOrder::Teiler)
        } else {
            list_results_sql(BestOrder::Punkte)
        };
        let mut stmt = self.conn.prepare(sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![competition_id], map_entry_result)
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        assign_entry_ranks(&mut out);
        Ok(out)
    }

    /// All series for an entry (chronological), with `isBest` by competition scoring mode.
    pub fn list_entry_series(&self, entry_id: &str) -> Result<Vec<SeriesResultSummary>, String> {
        let (scoring_mode, series) = self.load_entry_series(entry_id, false)?;
        let _ = scoring_mode;
        Ok(series)
    }

    pub fn get_entry_result(&self, entry_id: &str) -> Result<Option<EntryResultDetail>, String> {
        let meta = self
            .conn
            .query_row(
                "SELECT e.id, e.competition_id, e.person_id, e.start_order, e.status,
                        p.first_name, p.last_name, p.club,
                        c.name, c.scoring_mode, c.max_shots
                 FROM competition_entries e
                 JOIN people p ON p.id = e.person_id
                 JOIN competitions c ON c.id = e.competition_id
                 WHERE e.id = ?1",
                params![entry_id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, Option<String>>(5)?,
                        r.get::<_, Option<String>>(6)?,
                        r.get::<_, Option<String>>(7)?,
                        r.get::<_, String>(8)?,
                        r.get::<_, String>(9)?,
                        r.get::<_, i64>(10)?,
                    ))
                },
            )
            .optional_err()?;

        let Some((
            entry_id,
            competition_id,
            person_id,
            start_order,
            status,
            first_name,
            last_name,
            club,
            competition_name,
            scoring_mode,
            max_shots,
        )) = meta
        else {
            return Ok(None);
        };

        let (_, series) = self.load_entry_series(&entry_id, true)?;
        let best = series.iter().find(|s| s.is_best);

        let summary = EntryResultSummary {
            entry_id,
            competition_id,
            person_id,
            start_order,
            status,
            first_name,
            last_name,
            club,
            session_id: best.map(|s| s.session_id.clone()),
            session_ended_at: best.and_then(|s| s.ended_at.clone()),
            shot_count: best.map(|s| s.shot_count).unwrap_or(0),
            punkte_total: best.map(|s| s.punkte_total).unwrap_or(0.0),
            teiler_sum: best.map(|s| s.teiler_sum).unwrap_or(0.0),
            teiler_avg: best.map(|s| s.teiler_avg).unwrap_or(0.0),
            rank_punkte: None,
            rank_teiler: None,
        };

        let shots = best.map(|s| s.shots.clone()).unwrap_or_default();

        Ok(Some(EntryResultDetail {
            summary,
            competition_name,
            scoring_mode,
            max_shots,
            shots,
            series,
        }))
    }

    fn load_entry_series(
        &self,
        entry_id: &str,
        include_shots: bool,
    ) -> Result<(String, Vec<SeriesResultSummary>), String> {
        let scoring_mode: String = self
            .conn
            .query_row(
                "SELECT c.scoring_mode
                 FROM competition_entries e
                 JOIN competitions c ON c.id = e.competition_id
                 WHERE e.id = ?1",
                params![entry_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        let mut stmt = self
            .conn
            .prepare(
                "SELECT s.id, s.started_at, s.ended_at,
                        COALESCE(agg.shot_count, 0),
                        COALESCE(agg.punkte_total, 0),
                        COALESCE(agg.teiler_sum, 0),
                        COALESCE(agg.teiler_avg, 0)
                 FROM sessions s
                 LEFT JOIN (
                   SELECT session_id,
                          COUNT(*) AS shot_count,
                          COALESCE(SUM(score), 0) AS punkte_total,
                          COALESCE(SUM(CAST(distance_raw AS REAL) / 10.0), 0) AS teiler_sum,
                          COALESCE(AVG(CAST(distance_raw AS REAL) / 10.0), 0) AS teiler_avg
                   FROM shots
                   GROUP BY session_id
                 ) agg ON agg.session_id = s.id
                 WHERE s.entry_id = ?1
                 ORDER BY s.started_at ASC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![entry_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, f64>(4)?,
                    r.get::<_, f64>(5)?,
                    r.get::<_, f64>(6)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut series = Vec::new();
        for (i, row) in rows.enumerate() {
            let (session_id, started_at, ended_at, shot_count, punkte_total, teiler_sum, teiler_avg) =
                row.map_err(|e| e.to_string())?;
            let series_index = (i as i64) + 1;
            let shots = if include_shots {
                self.list_session_ui_shots(&session_id)?
            } else {
                Vec::new()
            };
            series.push(SeriesResultSummary {
                session_id,
                series_index,
                started_at,
                ended_at,
                shot_count,
                punkte_total,
                teiler_sum,
                teiler_avg,
                is_best: false,
                is_nachkauf: series_index > 1,
                shots,
            });
        }

        mark_best_series(&mut series, scoring_mode == "teiler");
        Ok((scoring_mode, series))
    }

    pub fn list_session_ui_shots(&self, session_id: &str) -> Result<Vec<ResultShot>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT shot_index, value_raw, distance_raw, x, y, score
                 FROM shots
                 WHERE session_id = ?1
                 ORDER BY shot_index ASC, session_sequence ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![session_id], |r| {
                let shot_index: i32 = r.get(0)?;
                let value_raw: i32 = r.get(1)?;
                let distance_raw: i32 = r.get(2)?;
                let x: i32 = r.get(3)?;
                let y: i32 = r.get(4)?;
                let score: f64 = r.get(5)?;
                Ok((shot_index, value_raw, distance_raw, x, y, score))
            })
            .map_err(|e| e.to_string())?;

        let mut out = Vec::new();
        let mut series_total = 0.0_f64;
        let mut series_teiler_total = 0.0_f64;
        for row in rows {
            let (shot_index, value_raw, distance_raw, x, y, score) =
                row.map_err(|e| e.to_string())?;
            let distance_display = distance_raw as f64 / 10.0;
            series_total += score;
            series_teiler_total += distance_display;
            out.push(ResultShot {
                shot_index: shot_index as u32,
                value_raw,
                distance_raw,
                x,
                y,
                value_display: score,
                distance_display,
                series_total,
                series_teiler_total,
            });
        }
        Ok(out)
    }
}

enum BestOrder {
    Punkte,
    Teiler,
}

fn list_results_sql(order: BestOrder) -> &'static str {
    // Prefer sessions with shots; then best by mode; tie-break latest start.
    match order {
        BestOrder::Punkte => {
            "SELECT e.id, e.competition_id, e.person_id, e.start_order, e.status,
                    p.first_name, p.last_name, p.club,
                    s.id, s.ended_at,
                    COALESCE(agg.shot_count, 0),
                    COALESCE(agg.punkte_total, 0),
                    COALESCE(agg.teiler_sum, 0),
                    COALESCE(agg.teiler_avg, 0)
             FROM competition_entries e
             JOIN people p ON p.id = e.person_id
             LEFT JOIN sessions s ON s.id = (
               SELECT s2.id FROM sessions s2
               LEFT JOIN (
                 SELECT session_id,
                        COUNT(*) AS shot_count,
                        COALESCE(SUM(score), 0) AS punkte_total
                 FROM shots
                 GROUP BY session_id
               ) a ON a.session_id = s2.id
               WHERE s2.entry_id = e.id
               ORDER BY CASE WHEN COALESCE(a.shot_count, 0) = 0 THEN 1 ELSE 0 END ASC,
                        COALESCE(a.punkte_total, -1e99) DESC,
                        s2.started_at DESC
               LIMIT 1
             )
             LEFT JOIN (
               SELECT session_id,
                      COUNT(*) AS shot_count,
                      COALESCE(SUM(score), 0) AS punkte_total,
                      COALESCE(SUM(CAST(distance_raw AS REAL) / 10.0), 0) AS teiler_sum,
                      COALESCE(AVG(CAST(distance_raw AS REAL) / 10.0), 0) AS teiler_avg
               FROM shots
               GROUP BY session_id
             ) agg ON agg.session_id = s.id
             WHERE e.competition_id = ?1
             ORDER BY e.start_order ASC"
        }
        BestOrder::Teiler => {
            "SELECT e.id, e.competition_id, e.person_id, e.start_order, e.status,
                    p.first_name, p.last_name, p.club,
                    s.id, s.ended_at,
                    COALESCE(agg.shot_count, 0),
                    COALESCE(agg.punkte_total, 0),
                    COALESCE(agg.teiler_sum, 0),
                    COALESCE(agg.teiler_avg, 0)
             FROM competition_entries e
             JOIN people p ON p.id = e.person_id
             LEFT JOIN sessions s ON s.id = (
               SELECT s2.id FROM sessions s2
               LEFT JOIN (
                 SELECT session_id,
                        COUNT(*) AS shot_count,
                        COALESCE(AVG(CAST(distance_raw AS REAL) / 10.0), 0) AS teiler_avg
                 FROM shots
                 GROUP BY session_id
               ) a ON a.session_id = s2.id
               WHERE s2.entry_id = e.id
               ORDER BY CASE WHEN COALESCE(a.shot_count, 0) = 0 THEN 1 ELSE 0 END ASC,
                        COALESCE(a.teiler_avg, 1e99) ASC,
                        s2.started_at DESC
               LIMIT 1
             )
             LEFT JOIN (
               SELECT session_id,
                      COUNT(*) AS shot_count,
                      COALESCE(SUM(score), 0) AS punkte_total,
                      COALESCE(SUM(CAST(distance_raw AS REAL) / 10.0), 0) AS teiler_sum,
                      COALESCE(AVG(CAST(distance_raw AS REAL) / 10.0), 0) AS teiler_avg
               FROM shots
               GROUP BY session_id
             ) agg ON agg.session_id = s.id
             WHERE e.competition_id = ?1
             ORDER BY e.start_order ASC"
        }
    }
}

/// Mark the single best series; empty list is a no-op. Prefer series with shots.
fn mark_best_series(series: &mut [SeriesResultSummary], teiler: bool) {
    if series.is_empty() {
        return;
    }
    for s in series.iter_mut() {
        s.is_best = false;
    }
    let best_idx = series
        .iter()
        .enumerate()
        .filter(|(_, s)| s.shot_count > 0)
        .min_by(|(_, a), (_, b)| {
            if teiler {
                a.teiler_avg
                    .partial_cmp(&b.teiler_avg)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.series_index.cmp(&a.series_index))
            } else {
                b.punkte_total
                    .partial_cmp(&a.punkte_total)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.series_index.cmp(&a.series_index))
            }
        })
        .map(|(i, _)| i)
        .unwrap_or(series.len() - 1);
    series[best_idx].is_best = true;
}

fn map_entry_result(r: &rusqlite::Row<'_>) -> rusqlite::Result<EntryResultSummary> {
    Ok(EntryResultSummary {
        entry_id: r.get(0)?,
        competition_id: r.get(1)?,
        person_id: r.get(2)?,
        start_order: r.get(3)?,
        status: r.get(4)?,
        first_name: r.get(5)?,
        last_name: r.get(6)?,
        club: r.get(7)?,
        session_id: r.get(8)?,
        session_ended_at: r.get(9)?,
        shot_count: r.get(10)?,
        punkte_total: r.get(11)?,
        teiler_sum: r.get(12)?,
        teiler_avg: r.get(13)?,
        rank_punkte: None,
        rank_teiler: None,
    })
}

/// Assign 1-based ranks for both scoring modes. Entries without shots stay unranked.
fn assign_entry_ranks(rows: &mut [EntryResultSummary]) {
    let mut by_punkte: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.shot_count > 0)
        .map(|(i, _)| i)
        .collect();
    by_punkte.sort_by(|&a, &b| {
        rows[b]
            .punkte_total
            .partial_cmp(&rows[a].punkte_total)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| rows[a].start_order.cmp(&rows[b].start_order))
    });
    for (rank, &idx) in by_punkte.iter().enumerate() {
        rows[idx].rank_punkte = Some((rank + 1) as i64);
    }

    let mut by_teiler: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.shot_count > 0)
        .map(|(i, _)| i)
        .collect();
    by_teiler.sort_by(|&a, &b| {
        rows[a]
            .teiler_avg
            .partial_cmp(&rows[b].teiler_avg)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| rows[a].start_order.cmp(&rows[b].start_order))
    });
    for (rank, &idx) in by_teiler.iter().enumerate() {
        rows[idx].rank_teiler = Some((rank + 1) as i64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(
        id: &str,
        start_order: i64,
        shot_count: i64,
        punkte: f64,
        teiler_avg: f64,
    ) -> EntryResultSummary {
        EntryResultSummary {
            entry_id: id.into(),
            competition_id: "c1".into(),
            person_id: id.into(),
            start_order,
            status: "done".into(),
            first_name: Some("A".into()),
            last_name: Some(id.into()),
            club: None,
            session_id: None,
            session_ended_at: None,
            shot_count,
            punkte_total: punkte,
            teiler_sum: teiler_avg * shot_count as f64,
            teiler_avg,
            rank_punkte: None,
            rank_teiler: None,
        }
    }

    fn series(index: i64, shot_count: i64, punkte: f64, teiler_avg: f64) -> SeriesResultSummary {
        SeriesResultSummary {
            session_id: format!("s{index}"),
            series_index: index,
            started_at: None,
            ended_at: None,
            shot_count,
            punkte_total: punkte,
            teiler_sum: teiler_avg * shot_count as f64,
            teiler_avg,
            is_best: false,
            is_nachkauf: index > 1,
            shots: Vec::new(),
        }
    }

    #[test]
    fn entry_ranks_punkte_descending_teiler_ascending() {
        let mut rows = vec![
            summary("low", 1, 10, 90.0, 50.0),
            summary("high", 2, 10, 110.0, 80.0),
            summary("mid", 3, 10, 100.0, 20.0),
            summary("none", 4, 0, 0.0, 0.0),
        ];
        assign_entry_ranks(&mut rows);

        let by_id = |id: &str| rows.iter().find(|r| r.entry_id == id).unwrap();
        assert_eq!(by_id("high").rank_punkte, Some(1));
        assert_eq!(by_id("mid").rank_punkte, Some(2));
        assert_eq!(by_id("low").rank_punkte, Some(3));
        assert_eq!(by_id("none").rank_punkte, None);

        assert_eq!(by_id("mid").rank_teiler, Some(1)); // lowest teiler
        assert_eq!(by_id("low").rank_teiler, Some(2));
        assert_eq!(by_id("high").rank_teiler, Some(3));
        assert_eq!(by_id("none").rank_teiler, None);
    }

    #[test]
    fn entry_ranks_tiebreak_by_start_order() {
        let mut rows = vec![
            summary("b", 2, 10, 100.0, 30.0),
            summary("a", 1, 10, 100.0, 30.0),
        ];
        assign_entry_ranks(&mut rows);
        let by_id = |id: &str| rows.iter().find(|r| r.entry_id == id).unwrap();
        assert_eq!(by_id("a").rank_punkte, Some(1));
        assert_eq!(by_id("b").rank_punkte, Some(2));
        assert_eq!(by_id("a").rank_teiler, Some(1));
        assert_eq!(by_id("b").rank_teiler, Some(2));
    }

    #[test]
    fn mark_best_series_punkte_picks_highest() {
        let mut s = vec![
            series(1, 10, 90.0, 40.0),
            series(2, 10, 105.0, 50.0),
            series(3, 10, 100.0, 30.0),
        ];
        mark_best_series(&mut s, false);
        assert!(!s[0].is_best);
        assert!(s[1].is_best);
        assert!(!s[2].is_best);
    }

    #[test]
    fn mark_best_series_teiler_picks_lowest() {
        let mut s = vec![
            series(1, 10, 90.0, 40.0),
            series(2, 10, 105.0, 25.0),
            series(3, 0, 0.0, 0.0),
        ];
        mark_best_series(&mut s, true);
        assert!(!s[0].is_best);
        assert!(s[1].is_best);
        assert!(!s[2].is_best);
    }
}

trait OptionalQuery<T> {
    fn optional_err(self) -> Result<Option<T>, String>;
}

impl<T> OptionalQuery<T> for Result<T, rusqlite::Error> {
    fn optional_err(self) -> Result<Option<T>, String> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}
