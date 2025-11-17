use nauto_plugin_sdk::{export_plugin, PluginMetadata, STANDARD_CAPABILITIES};

export_plugin!(PluginMetadata {
    vendor: "VendorX Experimental",
    device_type: "CiscoIos",
    capabilities: STANDARD_CAPABILITIES
});
