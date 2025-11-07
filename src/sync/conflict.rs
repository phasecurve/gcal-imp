use crate::calendar::Event;

#[derive(Debug, Clone, PartialEq)]
pub enum ResolutionStrategy {
    ServerWins,
    LocalWins,
    Merge,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    pub event_id: String,
    pub local: Event,
    pub remote: Event,
}

impl Conflict {
    pub fn new(local: Event, remote: Event) -> Self {
        Self {
            event_id: local.id.clone(),
            local,
            remote,
        }
    }
}

pub fn detect_conflict(local: &Event, remote: &Event) -> Option<Conflict> {
    if local.id != remote.id {
        return None;
    }

    if local.last_modified == remote.last_modified {
        return None;
    }

    if has_local_modifications(local) && remote.last_modified > local.last_modified {
        return Some(Conflict::new(local.clone(), remote.clone()));
    }

    None
}

pub fn resolve_conflict(
    local: &Event,
    remote: &Event,
    strategy: ResolutionStrategy,
) -> Event {
    match strategy {
        ResolutionStrategy::ServerWins => remote.clone(),
        ResolutionStrategy::LocalWins => local.clone(),
        ResolutionStrategy::Merge => merge_events(local, remote),
    }
}

fn has_local_modifications(event: &Event) -> bool {
    event.description.as_ref()
        .map(|d| d.contains("_local_modified"))
        .unwrap_or(false)
}

fn merge_events(local: &Event, remote: &Event) -> Event {
    let mut merged = remote.clone();

    if local.title != remote.title && !local.title.is_empty() {
        merged.title = local.title.clone();
    }

    if local.description.is_some() && local.description != remote.description {
        merged.description = local.description.clone();
    }

    if local.location.is_some() && local.location != remote.location {
        merged.location = local.location.clone();
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::calendar::EventStatus;

    fn create_event(id: &str, title: &str, last_modified_seconds: i64) -> Event {
        use chrono::TimeZone;
        let start = Utc::now();
        Event {
            id: id.to_string(),
            calendar_id: "primary".to_string(),
            title: title.to_string(),
            description: None,
            location: None,
            start,
            end: start + chrono::Duration::hours(1),
            all_day: false,
            attendees: vec![],
            reminders: vec![],
            status: EventStatus::Confirmed,
            last_modified: Utc.timestamp_opt(last_modified_seconds, 0).unwrap(),
            html_link: None,
        }
    }

    fn create_event_with_local_modification(id: &str, title: &str, last_modified_seconds: i64) -> Event {
        let mut event = create_event(id, title, last_modified_seconds);
        event.description = Some("_local_modified".to_string());
        event
    }

    #[test]
    fn no_conflict_when_timestamps_match() {
        let local = create_event("event1", "Meeting", 100);
        let remote = create_event("event1", "Meeting", 100);

        let conflict = detect_conflict(&local, &remote);

        assert!(conflict.is_none());
    }

    #[test]
    fn no_conflict_when_no_local_modifications() {
        let local = create_event("event1", "Meeting", 100);
        let remote = create_event("event1", "Meeting Updated", 150);

        let conflict = detect_conflict(&local, &remote);

        assert!(conflict.is_none());
    }

    #[test]
    fn conflict_detected_when_both_modified() {
        let local = create_event_with_local_modification("event1", "Local Title", 100);
        let remote = create_event("event1", "Remote Title", 150);

        let conflict = detect_conflict(&local, &remote);

        assert!(conflict.is_some());
        let c = conflict.unwrap();
        assert_eq!(c.event_id, "event1");
        assert_eq!(c.local.title, "Local Title");
        assert_eq!(c.remote.title, "Remote Title");
    }

    #[test]
    fn server_wins_uses_remote_version() {
        let local = create_event("event1", "Local", 100);
        let remote = create_event("event1", "Remote", 150);

        let resolved = resolve_conflict(&local, &remote, ResolutionStrategy::ServerWins);

        assert_eq!(resolved.title, "Remote");
    }

    #[test]
    fn local_wins_preserves_local_changes() {
        let local = create_event("event1", "Local", 100);
        let remote = create_event("event1", "Remote", 150);

        let resolved = resolve_conflict(&local, &remote, ResolutionStrategy::LocalWins);

        assert_eq!(resolved.title, "Local");
    }

    #[test]
    fn merge_strategy_combines_non_conflicting_fields() {
        let mut local = create_event("event1", "Local Title", 100);
        local.description = Some("Local description".to_string());

        let mut remote = create_event("event1", "Remote Title", 150);
        remote.location = Some("Remote location".to_string());

        let resolved = resolve_conflict(&local, &remote, ResolutionStrategy::Merge);

        assert_eq!(resolved.title, "Local Title");
        assert_eq!(resolved.description, Some("Local description".to_string()));
        assert_eq!(resolved.location, Some("Remote location".to_string()));
    }

    #[test]
    fn merge_strategy_prefers_non_empty_local_title() {
        let local = create_event("event1", "Local Title", 100);
        let remote = create_event("event1", "", 150);

        let resolved = resolve_conflict(&local, &remote, ResolutionStrategy::Merge);

        assert_eq!(resolved.title, "Local Title");
    }

    #[test]
    fn merge_strategy_uses_remote_as_base() {
        let local = create_event("event1", "Local", 100);
        let remote = create_event("event1", "Remote", 150);

        let resolved = resolve_conflict(&local, &remote, ResolutionStrategy::Merge);

        assert_eq!(resolved.last_modified, remote.last_modified);
    }
}
