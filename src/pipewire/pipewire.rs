//! PipeWire sample rate control module
//!
//! This module provides functionality to force PipeWire's sample rate
//! to match the currently playing song in MPD for bit-perfect playback.

use crate::logging::log_pipewire_operation;
use log::{debug, info, warn};
use pipewire::{
    context::ContextBox,
    main_loop::MainLoopBox,
    metadata::Metadata,
    properties::PropertiesBox,
    registry::GlobalObject,
    types::ObjectType,
};
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// The metadata name used by PipeWire for global settings
const SETTINGS_METADATA_NAME: &str = "settings";

/// Property key for forcing the clock rate
const CLOCK_FORCE_RATE_KEY: &str = "clock.force-rate";

/// Timeout for discovering PipeWire objects
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(2);

/// Timeout for sync operations
const SYNC_TIMEOUT: Duration = Duration::from_millis(500);

/// Iteration step for the main loop
const LOOP_ITERATION_STEP: Duration = Duration::from_millis(10);

/// Forces PipeWire to use a specific sample rate via the settings metadata.
///
/// This function connects to PipeWire, finds the "settings" metadata object,
/// and sets the `clock.force-rate` property to the desired rate.
///
/// # Arguments
/// * `rate` - The sample rate to force. Use 0 to reset to automatic rate selection.
///
/// # Returns
/// * `Ok(())` if the rate was set successfully
/// * `Err(String)` with an error message if something went wrong
pub fn set_sample_rate(rate: u32) -> Result<(), String> {
    let result = set_sample_rate_inner(rate);

    // Log the operation result
    let operation = if rate == 0 {
        "reset_sample_rate"
    } else {
        "set_sample_rate"
    };

    match &result {
        Ok(()) => {
            let details = if rate == 0 {
                "automatic".to_string()
            } else {
                format!("{} Hz", rate)
            };
            log_pipewire_operation(operation, true, Some(&details));
        }
        Err(e) => {
            log_pipewire_operation(operation, false, Some(e));
        }
    }

    result
}

fn set_sample_rate_inner(rate: u32) -> Result<(), String> {
    // Initialize PipeWire library
    pipewire::init();

    // Create the main loop
    let mainloop = MainLoopBox::new(None)
        .map_err(|e| format!("Failed to create PipeWire MainLoop: {e}"))?;

    // Create context from the main loop
    let context = ContextBox::new(mainloop.loop_(), None)
        .map_err(|e| format!("Failed to create PipeWire Context: {e}"))?;

    // Connect to the PipeWire server
    let core = context
        .connect(None)
        .map_err(|e| format!("Failed to connect to PipeWire Core: {e}"))?;

    // Get the registry to enumerate objects
    let registry = core
        .get_registry()
        .map_err(|e| format!("Failed to get PipeWire registry: {e}"))?;

    // Store found metadata global for later binding (using to_owned())
    let found_global: Rc<RefCell<Option<GlobalObject<PropertiesBox>>>> =
        Rc::new(RefCell::new(None));
    let found_global_clone = found_global.clone();

    // Register listener for global objects
    let _registry_listener = registry
        .add_listener_local()
        .global(move |global| {
            // Look for metadata objects with name "settings"
            if global.type_ == ObjectType::Metadata
                && let Some(props) = global.props.as_ref()
                && props.get("metadata.name") == Some(SETTINGS_METADATA_NAME)
            {
                // Store an owned copy of the global for later binding
                *found_global_clone.borrow_mut() = Some(global.to_owned());
            }
        })
        .register();

    // Run the loop to discover objects
    let start = std::time::Instant::now();
    while found_global.borrow().is_none() && start.elapsed() < DISCOVERY_TIMEOUT {
        mainloop.loop_().iterate(LOOP_ITERATION_STEP);
    }

    // Check if we found the settings metadata
    let global_ref = found_global.borrow();
    let global = global_ref
        .as_ref()
        .ok_or_else(|| "Timeout waiting for PipeWire settings metadata".to_string())?;

    // Bind to the metadata object
    let metadata: Metadata = registry
        .bind(global)
        .map_err(|e| format!("Failed to bind metadata: {e}"))?;

    // Set clock.force-rate (subject 0 = global settings)
    let rate_str = rate.to_string();
    metadata.set_property(0, CLOCK_FORCE_RATE_KEY, None, Some(&rate_str));

    if rate == 0 {
        debug!("Reset PipeWire sample rate to automatic");
    } else {
        info!("Set PipeWire sample rate to {rate} Hz");
    }

    // Sync to ensure the property change is flushed to the server
    let done = Rc::new(Cell::new(false));
    let done_clone = done.clone();

    let _core_listener = core
        .add_listener_local()
        .done(move |_id, _seq| {
            done_clone.set(true);
        })
        .register();

    // Trigger a sync - this will cause a done event when all prior commands are processed
    core.sync(0).map_err(|e| format!("Failed to sync: {e}"))?;

    // Drop the borrow before running the loop
    drop(global_ref);

    // Wait for the done event
    let sync_start = std::time::Instant::now();
    while !done.get() && sync_start.elapsed() < SYNC_TIMEOUT {
        mainloop.loop_().iterate(LOOP_ITERATION_STEP);
    }

    if !done.get() {
        warn!("PipeWire sync timed out, property may not be applied");
    }

    Ok(())
}

/// Resets PipeWire sample rate to automatic selection.
///
/// This clears the `clock.force-rate` property, allowing PipeWire
/// to automatically select the best sample rate.
pub fn reset_sample_rate() -> Result<(), String> {
    set_sample_rate(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires a running PipeWire instance
    fn test_set_and_reset_rate() {
        // Set a sample rate
        let result = set_sample_rate(48000);
        assert!(result.is_ok(), "Failed to set sample rate: {:?}", result);

        // Reset to automatic
        let result = reset_sample_rate();
        assert!(result.is_ok(), "Failed to reset sample rate: {:?}", result);
    }

    #[test]
    #[ignore] // Requires a running PipeWire instance
    fn test_common_sample_rates() {
        let rates = [44100, 48000, 88200, 96000, 176400, 192000];

        for rate in rates {
            let result = set_sample_rate(rate);
            assert!(
                result.is_ok(),
                "Failed to set sample rate {}: {:?}",
                rate,
                result
            );
        }

        // Clean up
        let _ = reset_sample_rate();
    }
}
