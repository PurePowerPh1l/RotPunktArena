use crate::db::{
    Competition, CompetitionEntry, CompetitionTeam, CreateCompetition, CreatePerson,
    EntryResultDetail, EntryResultSummary, Person, SeriesResultSummary, TeamResultSummary,
};
use crate::engine::StandEngine;
use std::sync::Arc;

#[tauri::command]
pub fn list_people(
    engine: tauri::State<'_, Arc<StandEngine>>,
    query: Option<String>,
    include_archived: Option<bool>,
) -> Result<Vec<Person>, String> {
    engine.with_db(|db| db.list_people(query.as_deref(), include_archived.unwrap_or(false)))
}

#[tauri::command]
pub fn create_person(
    engine: tauri::State<'_, Arc<StandEngine>>,
    person: CreatePerson,
) -> Result<Person, String> {
    engine.with_db(|db| db.create_person(person))
}

#[tauri::command]
pub fn set_person_archived(
    engine: tauri::State<'_, Arc<StandEngine>>,
    id: String,
    archived: bool,
) -> Result<Person, String> {
    engine.with_db(|db| db.set_person_archived(&id, archived))
}

#[tauri::command]
pub fn update_person(
    engine: tauri::State<'_, Arc<StandEngine>>,
    id: String,
    person: CreatePerson,
) -> Result<Person, String> {
    engine.with_db(|db| db.update_person(&id, person))
}

#[tauri::command]
pub fn delete_person(
    engine: tauri::State<'_, Arc<StandEngine>>,
    id: String,
) -> Result<(), String> {
    engine.with_db(|db| db.delete_person(&id))
}

#[tauri::command]
pub fn list_competitions(
    engine: tauri::State<'_, Arc<StandEngine>>,
    include_archived: Option<bool>,
) -> Result<Vec<Competition>, String> {
    engine.with_db(|db| db.list_competitions(include_archived.unwrap_or(false)))
}

#[tauri::command]
pub fn create_competition(
    engine: tauri::State<'_, Arc<StandEngine>>,
    competition: CreateCompetition,
) -> Result<Competition, String> {
    engine.with_db(|db| db.create_competition(competition))
}

#[tauri::command]
pub fn update_competition(
    engine: tauri::State<'_, Arc<StandEngine>>,
    id: String,
    competition: CreateCompetition,
) -> Result<Competition, String> {
    engine.with_db(|db| db.update_competition(&id, competition))
}

#[tauri::command]
pub fn set_competition_status(
    engine: tauri::State<'_, Arc<StandEngine>>,
    id: String,
    status: String,
) -> Result<Competition, String> {
    engine.with_db(|db| db.set_competition_status(&id, &status))
}

#[tauri::command]
pub fn create_from_competition(
    engine: tauri::State<'_, Arc<StandEngine>>,
    source_id: String,
    name: Option<String>,
    date: Option<String>,
    as_template: Option<bool>,
    copy_entries: Option<bool>,
) -> Result<Competition, String> {
    engine.with_db(|db| {
        db.create_from_competition(
            &source_id,
            name.as_deref(),
            date.as_deref(),
            as_template.unwrap_or(false),
            copy_entries.unwrap_or(true),
        )
    })
}

#[tauri::command]
pub fn set_competition_team_settings(
    engine: tauri::State<'_, Arc<StandEngine>>,
    id: String,
    team_scoring_enabled: bool,
    team_count: i64,
) -> Result<Competition, String> {
    engine.with_db(|db| db.set_competition_team_settings(&id, team_scoring_enabled, team_count))
}

#[tauri::command]
pub fn list_entries(
    engine: tauri::State<'_, Arc<StandEngine>>,
    competition_id: String,
) -> Result<Vec<CompetitionEntry>, String> {
    engine.with_db(|db| db.list_entries(&competition_id))
}

#[tauri::command]
pub fn add_entry(
    engine: tauri::State<'_, Arc<StandEngine>>,
    competition_id: String,
    person_id: String,
) -> Result<CompetitionEntry, String> {
    engine.with_db(|db| db.add_entry(&competition_id, &person_id))
}

#[tauri::command]
pub fn reorder_entries(
    engine: tauri::State<'_, Arc<StandEngine>>,
    competition_id: String,
    entry_ids: Vec<String>,
) -> Result<Vec<CompetitionEntry>, String> {
    engine.with_db(|db| db.reorder_entries(&competition_id, &entry_ids))
}

#[tauri::command]
pub fn set_entry_status(
    engine: tauri::State<'_, Arc<StandEngine>>,
    entry_id: String,
    status: String,
) -> Result<CompetitionEntry, String> {
    engine.with_db(|db| db.set_entry_status(&entry_id, &status))
}

/// Deprecated no-op: Nachkauf series counter is incremented on start, not via this command.
#[tauri::command]
pub fn set_entry_nachkauf(
    engine: tauri::State<'_, Arc<StandEngine>>,
    entry_id: String,
    nachkauf_purchased: i64,
) -> Result<CompetitionEntry, String> {
    engine.with_db(|db| db.set_entry_nachkauf(&entry_id, nachkauf_purchased))
}

#[tauri::command]
pub fn remove_entry(
    engine: tauri::State<'_, Arc<StandEngine>>,
    entry_id: String,
) -> Result<(), String> {
    engine.with_db(|db| db.remove_entry(&entry_id))
}

#[tauri::command]
pub fn clone_entries(
    engine: tauri::State<'_, Arc<StandEngine>>,
    from_competition_id: String,
    to_competition_id: String,
) -> Result<Vec<CompetitionEntry>, String> {
    engine.with_db(|db| db.clone_entries_from(&from_competition_id, &to_competition_id))
}

#[tauri::command]
pub fn list_competition_results(
    engine: tauri::State<'_, Arc<StandEngine>>,
    competition_id: String,
) -> Result<Vec<EntryResultSummary>, String> {
    engine.with_db(|db| db.list_competition_results(&competition_id))
}

#[tauri::command]
pub fn get_entry_result(
    engine: tauri::State<'_, Arc<StandEngine>>,
    entry_id: String,
) -> Result<Option<EntryResultDetail>, String> {
    engine.with_db(|db| db.get_entry_result(&entry_id))
}

#[tauri::command]
pub fn list_entry_series(
    engine: tauri::State<'_, Arc<StandEngine>>,
    entry_id: String,
) -> Result<Vec<SeriesResultSummary>, String> {
    engine.with_db(|db| db.list_entry_series(&entry_id))
}

#[tauri::command]
pub fn list_teams(
    engine: tauri::State<'_, Arc<StandEngine>>,
    competition_id: Option<String>,
    include_archived: Option<bool>,
) -> Result<Vec<CompetitionTeam>, String> {
    engine.with_db(|db| {
        db.list_teams(
            competition_id.as_deref(),
            include_archived.unwrap_or(false),
        )
    })
}

#[tauri::command]
pub fn list_known_team_names(
    engine: tauri::State<'_, Arc<StandEngine>>,
    include_archived: Option<bool>,
) -> Result<Vec<String>, String> {
    engine.with_db(|db| db.list_known_team_names(include_archived.unwrap_or(false)))
}

#[tauri::command]
pub fn create_team(
    engine: tauri::State<'_, Arc<StandEngine>>,
    name: String,
    // Ignored — teams are global. Kept for older frontend payloads.
    #[allow(unused_variables)]
    competition_id: Option<String>,
) -> Result<CompetitionTeam, String> {
    engine.with_db(|db| db.create_team(&name))
}

#[tauri::command]
pub fn rename_team(
    engine: tauri::State<'_, Arc<StandEngine>>,
    team_id: String,
    name: String,
) -> Result<CompetitionTeam, String> {
    engine.with_db(|db| db.rename_team(&team_id, &name))
}

#[tauri::command]
pub fn set_team_archived(
    engine: tauri::State<'_, Arc<StandEngine>>,
    team_id: String,
    archived: bool,
) -> Result<CompetitionTeam, String> {
    engine.with_db(|db| db.set_team_archived(&team_id, archived))
}

#[tauri::command]
pub fn remove_team(
    engine: tauri::State<'_, Arc<StandEngine>>,
    team_id: String,
) -> Result<(), String> {
    engine.with_db(|db| db.remove_team(&team_id))
}

#[tauri::command]
pub fn add_team_member(
    engine: tauri::State<'_, Arc<StandEngine>>,
    team_id: String,
    entry_id: String,
) -> Result<CompetitionTeam, String> {
    engine.with_db(|db| db.add_team_member(&team_id, &entry_id))
}

#[tauri::command]
pub fn remove_team_member(
    engine: tauri::State<'_, Arc<StandEngine>>,
    team_id: String,
    entry_id: String,
) -> Result<CompetitionTeam, String> {
    engine.with_db(|db| db.remove_team_member(&team_id, &entry_id))
}

#[tauri::command]
pub fn add_team_person(
    engine: tauri::State<'_, Arc<StandEngine>>,
    team_id: String,
    person_id: String,
) -> Result<CompetitionTeam, String> {
    engine.with_db(|db| db.add_team_person(&team_id, &person_id))
}

#[tauri::command]
pub fn remove_team_person(
    engine: tauri::State<'_, Arc<StandEngine>>,
    team_id: String,
    person_id: String,
) -> Result<CompetitionTeam, String> {
    engine.with_db(|db| db.remove_team_person(&team_id, &person_id))
}

#[tauri::command]
pub fn list_team_results(
    engine: tauri::State<'_, Arc<StandEngine>>,
    competition_id: String,
) -> Result<Vec<TeamResultSummary>, String> {
    engine.with_db(|db| db.list_team_results(&competition_id))
}
