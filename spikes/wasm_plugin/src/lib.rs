use nauto_plugin_sdk::{export_plugin, CapabilityMask, PluginMetadata};

export_plugin!(PluginMetadata {
    vendor: "VendorX Experimental",
    capabilities: CapabilityMask::COMMIT | CapabilityMask::ROLLBACK | CapabilityMask::DIFF
});

