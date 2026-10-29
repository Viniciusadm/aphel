use aphel_kernel::application::mail::MailLocale;
use aphel_kernel::application::notifications::NotificationView;
use aphel_kernel::application::notifications::texts::NotificationTexts;
use aphel_kernel::domain::notifications::NotificationKind;

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
