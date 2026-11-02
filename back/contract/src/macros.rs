#[macro_export]
macro_rules! contract {
    ($name:expr, $version:expr) => {
        $crate::Contract::new($name, $version)
    };
}

#[macro_export]
macro_rules! operation {
    ($key:expr) => {
        $crate::OperationKey::new($key)
    };
}

#[macro_export]
macro_rules! event {
    ($key:expr) => {
        $crate::EventKey::new($key)
    };
}

#[macro_export]
macro_rules! veto {
    ($key:expr) => {
        $crate::HookKey::new($key, $crate::HookKind::Veto)
    };
}

#[macro_export]
macro_rules! filter {
    ($key:expr) => {
        $crate::HookKey::new($key, $crate::HookKind::Filter)
    };
}

#[macro_export]
macro_rules! action {
    ($key:expr) => {
        $crate::HookKey::new($key, $crate::HookKind::Action)
    };
}

#[macro_export]
macro_rules! extension_point {
    ($key:expr) => {
        $crate::ExtensionPoint::new($key)
    };
}

#[macro_export]
macro_rules! reference {
    ($key:expr) => {
        $crate::ReferenceKind::new($key)
    };
}
