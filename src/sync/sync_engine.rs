use crate::calendar::Event;
use crate::storage::config::Config;
use crate::sync::google_api::{CalendarApi, DateRange, GoogleCalendarClient, CreatedEventInfo};
use crate::sync::google_auth::GoogleAuthenticator;
use chrono::NaiveDate;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("Authentication error: {0}")]
    AuthError(#[from] crate::sync::google_auth::AuthError),
    #[error("API error: {0}")]
    ApiError(#[from] crate::sync::google_api::ApiError),
}

pub struct SyncEngine {
    config: Config,
    auth: GoogleAuthenticator,
}

impl SyncEngine {
    pub fn new(config: Config) -> Self {
        let auth = GoogleAuthenticator::new(config.clone());
        Self { config, auth }
    }

    pub async fn fetch_events(
        &mut self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<Event>, SyncError> {
        let token = self.auth.get_valid_token().await?;

        let client = GoogleCalendarClient::new(token.access_token);

        let date_range = DateRange::new(start_date, end_date);

        let calendar_id = &self.config.calendars.default;
        let events = client.fetch_events(calendar_id, date_range).await?;

        Ok(events)
    }

    pub async fn fetch_events_around_date(
        &mut self,
        center_date: NaiveDate,
    ) -> Result<Vec<Event>, SyncError> {
        let days_past = self.config.sync.sync_past_days as i64;
        let days_future = self.config.sync.sync_future_days as i64;

        let start_date = center_date
            .checked_sub_days(chrono::Days::new(days_past as u64))
            .unwrap_or(center_date);

        let end_date = center_date
            .checked_add_days(chrono::Days::new(days_future as u64))
            .unwrap_or(center_date);

        self.fetch_events(start_date, end_date).await
    }

    pub async fn create_event(&mut self, event: &Event) -> Result<CreatedEventInfo, SyncError> {
        let token = self.auth.get_valid_token().await?;
        let client = GoogleCalendarClient::new(token.access_token);
        let calendar_id = &self.config.calendars.default;
        let created = client.create_event(calendar_id, event).await?;
        Ok(created)
    }

    pub async fn update_event(&mut self, event: &Event) -> Result<(), SyncError> {
        let token = self.auth.get_valid_token().await?;
        let client = GoogleCalendarClient::new(token.access_token);
        let calendar_id = &self.config.calendars.default;
        client.update_event(calendar_id, &event.id, event).await?;
        Ok(())
    }

    pub async fn delete_event(&mut self, event_id: &str) -> Result<(), SyncError> {
        let token = self.auth.get_valid_token().await?;
        let client = GoogleCalendarClient::new(token.access_token);
        let calendar_id = &self.config.calendars.default;
        client.delete_event(calendar_id, event_id).await?;
        Ok(())
    }
}
