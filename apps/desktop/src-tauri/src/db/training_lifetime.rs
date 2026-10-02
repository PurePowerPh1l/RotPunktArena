//! Stream every saved series into compact, stable lifetime display statistics.
use super::Database;
use std::collections::BTreeMap;
#[derive(Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingLifetime {
    pub key: String,
    pub session_count: u32,
    pub shot_count: i64,
    pub points_total: f64,
    pub sr: f64,
    pub previous_sr: f64,
    pub previous_points_total: f64,
    pub previous_shot_count: i64,
    #[serde(skip)]
    placement_total: f64,
}
impl Database {
    pub fn training_lifetime(&self) -> Result<Vec<TrainingLifetime>, String> {
        let mut statement = self.conn.prepare("SELECT s.person_id,s.shooter_name,t.shot_count,t.score FROM sessions s JOIN session_totals t ON t.session_id=s.id AND t.classification='scored' WHERE s.training_saved=1 AND s.competition_id IS NULL AND s.ended_at IS NOT NULL ORDER BY s.ended_at,s.id").map_err(|e| e.to_string())?;
        let mut rows = statement.query([]).map_err(|e| e.to_string())?;
        let mut groups: BTreeMap<String, TrainingLifetime> = BTreeMap::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let person: Option<String> = row.get(0).map_err(|e| e.to_string())?;
            let name: String = row.get(1).map_err(|e| e.to_string())?;
            let key = person
                .map(|id| format!("id:{id}"))
                .unwrap_or_else(|| format!("name:{}", name.trim().to_lowercase()));
            let count: i64 = row.get(2).map_err(|e| e.to_string())?;
            let points: f64 = row.get(3).map_err(|e| e.to_string())?;
            let g = groups
                .entry(key.clone())
                .or_insert_with(|| TrainingLifetime {
                    key,
                    ..Default::default()
                });
            g.previous_sr = g.sr;
            g.previous_shot_count = g.shot_count;
            g.previous_points_total = g.points_total;
            g.session_count += 1;
            g.shot_count += count;
            g.points_total += points;
            let normalized = if count > 0 {
                points * 10.0 / count as f64
            } else {
                0.0
            };
            if g.session_count <= 5 {
                g.placement_total += normalized;
                if g.session_count == 5 {
                    g.sr = ((g.placement_total / 5.0 - 75.0) / 35.0 * 5000.0).clamp(0.0, 5000.0);
                }
            } else {
                let raw = 32.0 * ((normalized - (75.0 + g.sr / 5000.0 * 35.0)) / 7.0).tanh();
                let delta = if raw > 0.0 {
                    raw * (1.0 - g.sr / 7200.0).max(0.32)
                } else {
                    raw * 0.78
                };
                g.sr = (g.sr + delta).clamp(0.0, 5000.0);
            }
        }
        Ok(groups.into_values().collect())
    }
}
