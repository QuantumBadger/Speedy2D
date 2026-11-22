use cfg_aliases::cfg_aliases;

fn main()
{
    // Needed for the internal `glutin_winit` module. As well as setting the
    // cfg aliases, this also declares them via `cargo:rustc-check-cfg`, which
    // is needed because Rust 1.77+ validates cfg names via `unexpected_cfgs`
    // (and this crate uses `#![deny(warnings)]`).
    cfg_aliases! {
        // Systems.
        android_platform: { target_os = "android" },
        wasm_platform: { target_family = "wasm" },
        macos_platform: { target_os = "macos" },
        ios_platform: { target_os = "ios" },
        apple: { any(ios_platform, macos_platform) },
        free_unix: { all(unix, not(apple), not(android_platform)) },

        // Native displays.
        x11_platform: { all(free_unix, not(wasm_platform)) },
        wayland_platform: { all(free_unix, not(wasm_platform)) },

        // Backends.
        egl_backend: { all(any(windows, unix), not(apple), not(wasm_platform)) },
        glx_backend: { all(x11_platform, not(wasm_platform)) },
        wgl_backend: { all(windows, not(wasm_platform)) },
        cgl_backend: { all(macos_platform, not(wasm_platform)) },
    }
}
