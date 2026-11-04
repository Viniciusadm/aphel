use aphel_kernel::application::mail::MailLocale;
use aphel_sdk::notifications::NotificationKind;
use aphel_sdk::notifications::NotificationTexts;
use aphel_sdk::notifications::NotificationView;

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
