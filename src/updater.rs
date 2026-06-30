#[cfg(target_os = "macos")]
mod platform {
    #![allow(unexpected_cfgs)]

    use std::sync::OnceLock;

    use objc::{
        class, msg_send,
        runtime::{BOOL, Class, Object},
        sel, sel_impl,
    };

    static SPARKLE_UPDATER: OnceLock<usize> = OnceLock::new();

    pub(crate) fn start() {
        let _ = SPARKLE_UPDATER.get_or_init(|| unsafe { start_sparkle().unwrap_or(0) });
    }

    unsafe fn start_sparkle() -> Option<usize> {
        if !unsafe { load_sparkle_framework() } {
            return None;
        }

        let controller_class = Class::get("SPUStandardUpdaterController")?;
        let controller: *mut Object = unsafe { msg_send![controller_class, alloc] };
        if controller.is_null() {
            return None;
        }

        let yes: BOOL = true;
        let nil: *mut Object = std::ptr::null_mut();
        let controller: *mut Object = unsafe {
            msg_send![
                controller,
                initWithStartingUpdater: yes
                updaterDelegate: nil
                userDriverDelegate: nil
            ]
        };

        if controller.is_null() {
            None
        } else {
            Some(controller as usize)
        }
    }

    unsafe fn load_sparkle_framework() -> bool {
        let bundle_class = class!(NSBundle);
        let main_bundle: *mut Object = unsafe { msg_send![bundle_class, mainBundle] };
        if main_bundle.is_null() {
            return false;
        }

        let private_frameworks_path: *mut Object =
            unsafe { msg_send![main_bundle, privateFrameworksPath] };
        if private_frameworks_path.is_null() {
            return false;
        }

        let sparkle_component = unsafe { ns_string("Sparkle.framework") };
        if sparkle_component.is_null() {
            return false;
        }

        let sparkle_path: *mut Object = unsafe {
            msg_send![private_frameworks_path, stringByAppendingPathComponent: sparkle_component]
        };
        unsafe {
            let _: () = msg_send![sparkle_component, release];
        }

        if sparkle_path.is_null() {
            return false;
        }

        let sparkle_bundle: *mut Object =
            unsafe { msg_send![bundle_class, bundleWithPath: sparkle_path] };
        if sparkle_bundle.is_null() {
            return false;
        }

        let loaded: BOOL = unsafe { msg_send![sparkle_bundle, load] };
        loaded
    }

    unsafe fn ns_string(value: &str) -> *mut Object {
        let ns_string: *mut Object = unsafe { msg_send![class!(NSString), alloc] };
        unsafe {
            msg_send![
                ns_string,
                initWithBytes: value.as_ptr()
                length: value.len()
                encoding: 4usize
            ]
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    pub(crate) fn start() {}
}

pub(crate) use platform::start;
