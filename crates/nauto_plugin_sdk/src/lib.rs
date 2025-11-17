use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct CapabilityMask: u32 {
        const COMMIT = 1 << 0;
        const ROLLBACK = 1 << 1;
        const DIFF = 1 << 2;
        const DRY_RUN = 1 << 3;
    }
}

impl CapabilityMask {
    pub const fn all_standard() -> Self {
        Self::COMMIT | Self::ROLLBACK | Self::DIFF | Self::DRY_RUN
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PluginMetadata {
    pub vendor: &'static str,
    pub capabilities: CapabilityMask,
}

#[macro_export]
macro_rules! export_plugin {
    ($meta:expr) => {
        const _PLUGIN_META: $crate::PluginMetadata = $meta;

        #[no_mangle]
        pub extern "C" fn plugin_vendor_ptr() -> *const u8 {
            _PLUGIN_META.vendor.as_ptr()
        }

        #[no_mangle]
        pub extern "C" fn plugin_vendor_len() -> usize {
            _PLUGIN_META.vendor.len()
        }

        #[no_mangle]
        pub extern "C" fn plugin_capabilities() -> u32 {
            _PLUGIN_META.capabilities.bits()
        }
    };
}
