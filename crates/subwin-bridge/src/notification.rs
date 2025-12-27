/// Severity or category for user-visible notifications.
///
/// This enum classifies notifications by their intent and visual styling,
/// allowing the UI to display them appropriately.
#[derive(Debug, Clone)]
pub enum NotificationType {
    /// Neutral informational message that does not indicate success or failure.
    Info,
    /// Indicates a successful operation or positive outcome.
    Success,
    /// Indicates a non-critical issue that the user should be aware of, but
    /// does not prevent normal operation.
    Warning,
    /// Indicates an error or failure that may affect functionality.
    Error,
}

/// A notification payload intended for the user interface.
#[derive(Debug, Clone)]
pub struct NotificationMessage {
    /// The type/severity of the notification, determining its visual style.
    pub notification_type: NotificationType,
    /// The text content to display to the user.
    pub message: String,
}
