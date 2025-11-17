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

pub const STANDARD_CAPABILITIES: CapabilityMask = CapabilityMask::from_bits_retain(
    CapabilityMask::COMMIT.bits()
        | CapabilityMask::ROLLBACK.bits()
        | CapabilityMask::DIFF.bits()
        | CapabilityMask::DRY_RUN.bits(),
);

impl CapabilityMask {
    pub fn all_standard() -> Self {
        STANDARD_CAPABILITIES
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PluginMetadata {
    pub vendor: &'static str,
    pub device_type: &'static str,
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

        #[no_mangle]
        pub extern "C" fn plugin_device_type_ptr() -> *const u8 {
            _PLUGIN_META.device_type.as_ptr()
        }

        #[no_mangle]
        pub extern "C" fn plugin_device_type_len() -> usize {
            _PLUGIN_META.device_type.len()
        }
    };
}
