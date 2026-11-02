use ::notifications::application::NotificationView;
use ::notifications::application::texts::NotificationTexts;
use ::notifications::domain::NotificationKind;
use aphel_kernel::application::mail::MailLocale;

pub(crate) const REMINDER_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("reminder");

pub(crate) struct ReminderNotificationTexts;

impl NotificationTexts for ReminderNotificationTexts {
    fn headline(
        &self,
        view: &NotificationView,
        _actor: &str,
        locale: MailLocale,
    ) -> Option<String> {
        (view.kind == REMINDER_NOTIFICATION).then(|| locale.pick("Lembrete", "Reminder").to_owned())
    }

    fn default_path(&self, view: &NotificationView) -> Option<&'static str> {
        (view.kind == REMINDER_NOTIFICATION).then_some("/inbox")
    }
}
