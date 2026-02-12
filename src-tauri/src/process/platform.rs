use crate::platform::{create_platform_manager, PlatformProcessManager};
use std::sync::OnceLock;

/// Platform-specific process manager instance
static PLATFORM_MANAGER: OnceLock<Box<dyn PlatformProcessManager>> = OnceLock::new();

/// Get the platform-specific process manager
pub fn get_platform_manager() -> &'static dyn PlatformProcessManager {
    PLATFORM_MANAGER
        .get_or_init(|| Box::new(create_platform_manager()))
        .as_ref()
}
