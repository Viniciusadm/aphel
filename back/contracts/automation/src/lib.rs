pub mod application;
pub mod domain;
pub type Trigger = std::sync::Arc<dyn application::AutomationTrigger>;
pub type Action = std::sync::Arc<dyn application::AutomationAction>;
pub trait AutomationBuilder {
    fn trigger(&mut self, trigger: Trigger) -> &mut Self;
    fn action(&mut self, action: Action) -> &mut Self;
    fn triggers(&mut self, triggers: impl IntoIterator<Item = Trigger>) -> &mut Self;
    fn actions(&mut self, actions: impl IntoIterator<Item = Action>) -> &mut Self;
}
impl AutomationBuilder for aphel_kernel::modules::AppBuilder {
    fn trigger(&mut self, trigger: Trigger) -> &mut Self {
        self.contribute::<dyn application::AutomationTrigger>(trigger)
    }
    fn action(&mut self, action: Action) -> &mut Self {
        self.contribute::<dyn application::AutomationAction>(action)
    }
    fn triggers(&mut self, triggers: impl IntoIterator<Item = Trigger>) -> &mut Self {
        for trigger in triggers {
            self.trigger(trigger);
        }
        self
    }
    fn actions(&mut self, actions: impl IntoIterator<Item = Action>) -> &mut Self {
        for action in actions {
            self.action(action);
        }
        self
    }
}
