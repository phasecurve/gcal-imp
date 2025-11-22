use crate::calendar::{Event, EventStatus};
use chrono::{DateTime, NaiveDate, Utc};
use thiserror::Error;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Request error: {0}")]
    RequestError(String),
    #[error("Calendar not found: {0}")]
    NotFound(String),
    #[error("Rate limit exceeded")]
    RateLimited,
    #[error("Authentication failed")]
    AuthenticationFailed,
    #[error("Parse error: {0}")]
    ParseError(String),
}

pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl DateRange {
    pub fn new(start: NaiveDate, end: NaiveDate) -> Self {
        Self { start, end }
    }

    pub fn days(&self) -> i64 {
        (self.end - self.start).num_days()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct GoogleEvent {
    id: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    location: Option<String>,
    start: GoogleDateTime,
    end: GoogleDateTime,
    status: Option<String>,
    updated: Option<String>,
    #[serde(rename = "htmlLink")]
    html_link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoogleDateTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EventListResponse {
    items: Option<Vec<GoogleEvent>>,
}

#[async_trait]
pub trait CalendarApi {
    async fn fetch_events(
        &self,
        calendar_id: &str,
        date_range: DateRange,
    ) -> Result<Vec<Event>, ApiError>;

    async fn create_event(
        &self,
        calendar_id: &str,
        event: &Event,
    ) -> Result<CreatedEventInfo, ApiError>;

    async fn update_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        event: &Event,
    ) -> Result<(), ApiError>;

    async fn delete_event(
        &self,
        calendar_id: &str,
        event_id: &str,
    ) -> Result<(), ApiError>;
}

pub struct GoogleCalendarClient {
    base_url: String,
    access_token: String,
    client: reqwest::Client,
}

#[derive(Debug, Clone)]
pub struct CreatedEventInfo {
    pub id: String,
    pub html_link: Option<String>,
}

impl GoogleCalendarClient {
    pub fn new(access_token: String) -> Self {
        Self {
            base_url: "https://www.googleapis.com/calendar/v3".to_string(),
            access_token,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    fn convert_from_google_event(&self, ge: GoogleEvent, calendar_id: &str) -> Result<Event, ApiError> {
        let start_str = ge.start.date_time
            .ok_or_else(|| ApiError::ParseError("Missing start dateTime".to_string()))?;
        let end_str = ge.end.date_time
            .ok_or_else(|| ApiError::ParseError("Missing end dateTime".to_string()))?;

        let start = DateTime::parse_from_rfc3339(&start_str)
            .map_err(|e| ApiError::ParseError(format!("Invalid start time: {}", e)))?
            .with_timezone(&Utc);

        let end = DateTime::parse_from_rfc3339(&end_str)
            .map_err(|e| ApiError::ParseError(format!("Invalid end time: {}", e)))?
            .with_timezone(&Utc);

        let status = match ge.status.as_deref() {
            Some("confirmed") => EventStatus::Confirmed,
            Some("tentative") => EventStatus::Tentative,
            Some("cancelled") => EventStatus::Cancelled,
            _ => EventStatus::Confirmed,
        };

        let last_modified = if let Some(updated) = ge.updated {
            DateTime::parse_from_rfc3339(&updated)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now())
        } else {
            Utc::now()
        };

        Ok(Event {
            id: ge.id.ok_or_else(|| ApiError::ParseError("Missing event id".to_string()))?,
            calendar_id: calendar_id.to_string(),
            title: ge.summary.unwrap_or_default(),
            description: ge.description,
            location: ge.location,
            start,
            end,
            all_day: false,
            attendees: vec![],
            reminders: vec![],
            status,
            last_modified,
            html_link: ge.html_link,
        })
    }

    fn convert_to_google_event(&self, event: &Event, include_id: bool) -> GoogleEvent {
        GoogleEvent {
            id: if include_id { Some(event.id.clone()) } else { None },
            summary: Some(event.title.clone()),
            description: event.description.clone(),
            location: event.location.clone(),
            start: GoogleDateTime {
                date_time: Some(event.start.to_rfc3339()),
                date: None,
            },
            end: GoogleDateTime {
                date_time: Some(event.end.to_rfc3339()),
                date: None,
            },
            status: Some(match event.status {
                EventStatus::Confirmed => "confirmed",
                EventStatus::Tentative => "tentative",
                EventStatus::Cancelled => "cancelled",
            }.to_string()),
            updated: Some(event.last_modified.to_rfc3339()),
            html_link: None,
        }
    }
}

#[async_trait]
impl CalendarApi for GoogleCalendarClient {
    async fn fetch_events(
        &self,
        calendar_id: &str,
        date_range: DateRange,
    ) -> Result<Vec<Event>, ApiError> {
        let time_min = date_range.start.and_hms_opt(0, 0, 0)
            .ok_or_else(|| ApiError::ParseError("Invalid start date".to_string()))?
            .and_utc()
            .to_rfc3339();

        let time_max = date_range.end.and_hms_opt(23, 59, 59)
            .ok_or_else(|| ApiError::ParseError("Invalid end date".to_string()))?
            .and_utc()
            .to_rfc3339();

        let url = format!("{}/calendars/{}/events", self.base_url, calendar_id);

        tracing::info!("Fetching events from {} to {}", date_range.start, date_range.end);

        let response = self.client
            .get(&url)
            .bearer_auth(&self.access_token)
            .query(&[
                ("timeMin", time_min.as_str()),
                ("timeMax", time_max.as_str()),
                ("singleEvents", "true"),
                ("orderBy", "startTime"),
            ])
            .send()
            .await?;

        let status = response.status();
        tracing::info!("Fetch events response status: {}", status);

        if status == 401 {
            tracing::error!("Authentication failed when fetching events");
            return Err(ApiError::AuthenticationFailed);
        }

        if status == 404 {
            tracing::error!("Calendar not found: {}", calendar_id);
            return Err(ApiError::NotFound(calendar_id.to_string()));
        }

        if status == 429 {
            tracing::warn!("Rate limit exceeded");
            return Err(ApiError::RateLimited);
        }

        if !status.is_success() {
            let body = response.text().await?;
            tracing::error!("Failed to fetch events. Status: {}, Body: {}", status, body);
            return Err(ApiError::RequestError(format!("Status {}: {}", status, body)));
        }

        let event_list: EventListResponse = response.json().await?;

        let events: Vec<Event> = event_list.items
            .unwrap_or_default()
            .into_iter()
            .filter_map(|ge| self.convert_from_google_event(ge, calendar_id).ok())
            .collect();

        tracing::info!("Fetched {} events successfully", events.len());
        Ok(events)
    }

    async fn create_event(
        &self,
        calendar_id: &str,
        event: &Event,
    ) -> Result<CreatedEventInfo, ApiError> {
        let url = format!("{}/calendars/{}/events", self.base_url, calendar_id);
        let google_event = self.convert_to_google_event(event, false);

        tracing::info!("Creating event: {} on {}", event.title, event.start);
        tracing::debug!("POST {} with payload: {:?}", url, google_event);

        let response = self.client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&google_event)
            .send()
            .await?;

        let status = response.status();
        tracing::info!("Create event response status: {}", status);

        if status == 401 {
            tracing::error!("Authentication failed when creating event");
            return Err(ApiError::AuthenticationFailed);
        }

        if !status.is_success() {
            let body = response.text().await?;
            tracing::error!("Failed to create event. Status: {}, Body: {}", status, body);
            return Err(ApiError::RequestError(format!("Status {}: {}", status, body)));
        }

        let created_event: GoogleEvent = response.json().await?;
        let id = created_event.id.unwrap_or_default();
        tracing::info!("Event created successfully with ID: {:?}", id);

        Ok(CreatedEventInfo {
            id,
            html_link: created_event.html_link,
        })
    }

    async fn update_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        event: &Event,
    ) -> Result<(), ApiError> {
        let url = format!("{}/calendars/{}/events/{}", self.base_url, calendar_id, event_id);
        let google_event = self.convert_to_google_event(event, true);

        tracing::info!("Updating event {}: {}", event_id, event.title);
        tracing::debug!("PUT {} with payload: {:?}", url, google_event);

        let response = self.client
            .put(&url)
            .bearer_auth(&self.access_token)
            .json(&google_event)
            .send()
            .await?;

        let status = response.status();
        tracing::info!("Update event response status: {}", status);

        if status == 401 {
            tracing::error!("Authentication failed when updating event {}", event_id);
            return Err(ApiError::AuthenticationFailed);
        }

        if status == 404 {
            tracing::error!("Event not found: {}", event_id);
            return Err(ApiError::NotFound(event_id.to_string()));
        }

        if !status.is_success() {
            let body = response.text().await?;
            tracing::error!("Failed to update event {}. Status: {}, Body: {}", event_id, status, body);
            return Err(ApiError::RequestError(format!("Status {}: {}", status, body)));
        }

        tracing::info!("Event {} updated successfully", event_id);
        Ok(())
    }

    async fn delete_event(
        &self,
        calendar_id: &str,
        event_id: &str,
    ) -> Result<(), ApiError> {
        let url = format!("{}/calendars/{}/events/{}", self.base_url, calendar_id, event_id);

        let response = self.client
            .delete(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        if response.status() == 401 {
            return Err(ApiError::AuthenticationFailed);
        }

        if response.status() == 404 {
            return Err(ApiError::NotFound(event_id.to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            return Err(ApiError::RequestError(format!("Status {}: {}", status, body)));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn date_range_calculates_days() {
        let range = DateRange::new(
            date(2025, 1, 1),
            date(2025, 1, 8),
        );

        assert_eq!(range.days(), 7);
    }

    #[test]
    fn date_range_same_day_is_zero_days() {
        let range = DateRange::new(
            date(2025, 1, 1),
            date(2025, 1, 1),
        );

        assert_eq!(range.days(), 0);
    }

    #[test]
    fn google_calendar_client_has_default_base_url() {
        let client = GoogleCalendarClient::new("token".to_string());

        assert_eq!(client.base_url, "https://www.googleapis.com/calendar/v3");
    }

    #[test]
    fn google_calendar_client_can_set_custom_base_url() {
        let client = GoogleCalendarClient::new("token".to_string())
            .with_base_url("http://localhost:8080".to_string());

        assert_eq!(client.base_url, "http://localhost:8080");
    }
}
