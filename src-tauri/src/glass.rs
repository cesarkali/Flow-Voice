//! "Vidro": Windows' own blur behind the overlay card, like the island mod's Liquid Glass. A
//! composition visual with the host backdrop brush (what's behind, blurred by DWM), clipped to
//! the card's rounded box, on a window of its own right under the overlay (the mouse goes through
//! it). On the webview's own window Windows only gave the backdrop's flat fallback color. The
//! tint, the bright edge and the light are CSS on the card. The blur is drawn by the compositor on the GPU, so it costs the app
//! nothing while dictating. Needs Windows' transparency effects (off, or with the energy saver,
//! the backdrop is a flat color: the card stays solid then).

use serde::Deserialize;
use tauri::AppHandle;

/// The card's box in CSS px relative to the overlay window, its corner radius and its opacity.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct GlassRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
    pub opacity: f64,
}

/// Whether the glass can show now (Windows, transparency effects on, no energy saver).
pub fn available() -> bool {
    #[cfg(windows)]
    {
        imp::available()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Puts the glass under the card (None: fades it out).
pub fn update(app: &AppHandle, rect: Option<GlassRect>) {
    #[cfg(windows)]
    imp::update(app, rect);
    #[cfg(not(windows))]
    let _ = (app, rect);
}

#[cfg(windows)]
mod imp {
    use super::GlassRect;
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tauri::{AppHandle, Manager};
    use windows::core::{Interface, HSTRING};
    use windows::Foundation::Numerics::{Vector2, Vector3};
    use windows::Foundation::TimeSpan;
    use windows::System::{DispatcherQueue, DispatcherQueueController};
    use windows::core::w;
    use windows::Win32::Foundation::{BOOL, COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowRect, RegisterClassW, SetLayeredWindowAttributes,
        SetWindowPos, ShowWindow, LWA_ALPHA, SWP_NOACTIVATE, SW_SHOWNOACTIVATE, WNDCLASSW, WS_EX_LAYERED,
        WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
    };
    use windows::Win32::System::WinRT::Composition::ICompositorDesktopInterop;
    use windows::Win32::System::WinRT::{CreateDispatcherQueueController, DispatcherQueueOptions, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT};
    use windows::UI::Composition::Desktop::DesktopWindowTarget;
    use windows::UI::Composition::{CompositionRoundedRectangleGeometry, Compositor, SpriteVisual};

    /// DWMWA_USE_HOSTBACKDROPBRUSH: lets the window's composition use the host backdrop.
    const USE_HOSTBACKDROPBRUSH: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(17);
    /// 100 ns ticks.
    const FADE_OUT: TimeSpan = TimeSpan { Duration: 1_800_000 };

    struct Glass {
        /// The overlay window it follows, and its own.
        owner: isize,
        hwnd: isize,
        placed: Option<(i32, i32, i32, i32)>,
        _queue: Option<DispatcherQueueController>,
        compositor: Compositor,
        _target: DesktopWindowTarget,
        visual: SpriteVisual,
        shape: CompositionRoundedRectangleGeometry,
        last: Option<(GlassRect, f64)>,
    }

    thread_local! {
        // Composition objects live on the thread that made them: the main (UI) thread.
        static GLASS: RefCell<Option<Glass>> = const { RefCell::new(None) };
    }
    static FAILED: AtomicBool = AtomicBool::new(false);

    fn dword(path: &str, name: &str) -> Option<u32> {
        let mut v = 0u32;
        let mut size = 4u32;
        let ok = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                &HSTRING::from(path),
                &HSTRING::from(name),
                RRF_RT_REG_DWORD,
                None,
                Some(&mut v as *mut u32 as *mut _),
                Some(&mut size),
            )
        };
        ok.is_ok().then_some(v)
    }

    pub fn available() -> bool {
        if FAILED.load(Ordering::Relaxed) {
            return false;
        }
        let transparency = dword(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "EnableTransparency") != Some(0);
        let mut power = SYSTEM_POWER_STATUS::default();
        let saver = unsafe { GetSystemPowerStatus(&mut power) }.is_ok() && power.SystemStatusFlag == 1;
        transparency && !saver
    }

    impl Drop for Glass {
        fn drop(&mut self) {
            let _ = unsafe { DestroyWindow(HWND(self.hwnd as *mut _)) };
        }
    }

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        DefWindowProcW(hwnd, msg, wp, lp)
    }

    /// The glass's own window: click-through, never activated, no redirection bitmap (composition
    /// only), as the mod's (`WS_EX_LAYERED` with full alpha keeps the mouse going through it).
    fn create_window() -> windows::core::Result<HWND> {
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = w!("FlowVoiceGlass");
            let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: instance.into(), lpszClassName: class, ..Default::default() };
            RegisterClassW(&wc); // 0 when already registered (a second glass after the overlay was made again)
            let hwnd = CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class,
                w!(""),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                instance,
                None,
            )?;
            SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA)?;
            Ok(hwnd)
        }
    }

    fn create(owner: HWND) -> windows::core::Result<Glass> {
        // The compositor needs a dispatcher queue on this thread (the webview may have one already).
        let queue = if DispatcherQueue::GetForCurrentThread().is_ok() {
            None
        } else {
            let options = DispatcherQueueOptions {
                dwSize: std::mem::size_of::<DispatcherQueueOptions>() as u32,
                threadType: DQTYPE_THREAD_CURRENT,
                apartmentType: DQTAT_COM_NONE,
            };
            Some(unsafe { CreateDispatcherQueueController(options)? })
        };
        let compositor = Compositor::new()?;
        let hwnd = create_window()?;
        let glass_hwnd = hwnd.0 as isize;
        let destroy = || {
            let _ = unsafe { DestroyWindow(HWND(glass_hwnd as *mut _)) };
        };
        let on = BOOL::from(true);
        // A window of an app that isn't from the Store shows Windows' blur only with this.
        if let Err(e) = unsafe {
            DwmSetWindowAttribute(hwnd, USE_HOSTBACKDROPBRUSH, &on as *const BOOL as *const _, std::mem::size_of::<BOOL>() as u32)
        } {
            destroy();
            return Err(e);
        }
        let interop: ICompositorDesktopInterop = compositor.cast()?;
        let target = match unsafe { interop.CreateDesktopWindowTarget(hwnd, BOOL::from(true)) } {
            Ok(t) => t,
            Err(e) => {
                destroy();
                return Err(e);
            }
        };
        let root = compositor.CreateContainerVisual()?;
        root.SetRelativeSizeAdjustment(Vector2 { X: 1.0, Y: 1.0 })?;
        target.SetRoot(&root)?;
        let visual = compositor.CreateSpriteVisual()?;
        visual.SetBrush(&compositor.CreateHostBackdropBrush()?)?;
        let shape = compositor.CreateRoundedRectangleGeometry()?;
        visual.SetClip(&compositor.CreateGeometricClipWithGeometry(&shape)?)?;
        visual.SetOpacity(0.0)?;
        root.Children()?.InsertAtTop(&visual)?;
        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
        Ok(Glass {
            owner: owner.0 as isize,
            hwnd: glass_hwnd,
            placed: None,
            _queue: queue,
            compositor,
            _target: target,
            visual,
            shape,
            last: None,
        })
    }

    fn apply(g: &mut Glass, rect: Option<GlassRect>, scale: f64) -> windows::core::Result<()> {
        let Some(r) = rect else {
            if g.last.take().is_some() {
                let fade = g.compositor.CreateScalarKeyFrameAnimation()?;
                fade.InsertKeyFrame(1.0, 0.0)?;
                fade.SetDuration(FADE_OUT)?;
                g.visual.StartAnimation(&HSTRING::from("Opacity"), &fade)?;
            }
            return Ok(());
        };
        let appearing = g.last.is_none();
        if appearing {
            g.visual.StopAnimation(&HSTRING::from("Opacity"))?;
        }
        // Over the overlay window's box, right under it (it may have moved to another monitor,
        // or been raised over the glass when shown).
        let mut wr = RECT::default();
        if unsafe { GetWindowRect(HWND(g.owner as *mut _), &mut wr) }.is_ok() {
            let place = (wr.left, wr.top, wr.right - wr.left, wr.bottom - wr.top);
            if appearing || g.placed != Some(place) {
                g.placed = Some(place);
                let _ = unsafe {
                    SetWindowPos(HWND(g.hwnd as *mut _), HWND(g.owner as *mut _), place.0, place.1, place.2, place.3, SWP_NOACTIVATE)
                };
            }
        }
        if g.last == Some((r, scale)) {
            return Ok(());
        }
        g.last = Some((r, scale));
        let px = |v: f64| (v * scale) as f32;
        g.visual.SetOffset(Vector3 { X: px(r.x), Y: px(r.y), Z: 0.0 })?;
        let size = Vector2 { X: px(r.width), Y: px(r.height) };
        g.visual.SetSize(size)?;
        g.shape.SetSize(size)?;
        let radius = px(r.radius).min(size.X / 2.0).min(size.Y / 2.0).max(0.0);
        g.shape.SetCornerRadius(Vector2 { X: radius, Y: radius })?;
        g.visual.SetOpacity(r.opacity.clamp(0.0, 1.0) as f32)?;
        Ok(())
    }

    pub fn update(app: &AppHandle, rect: Option<GlassRect>) {
        if FAILED.load(Ordering::Relaxed) {
            return;
        }
        let app2 = app.clone();
        let _ = app.run_on_main_thread(move || {
            let Some(w) = app2.get_webview_window("overlay") else { return };
            let (Ok(hwnd), Ok(scale)) = (w.hwnd(), w.scale_factor()) else { return };
            let hwnd = HWND(hwnd.0 as *mut _);
            GLASS.with(|cell| {
                let mut cell = cell.borrow_mut();
                if cell.as_ref().is_some_and(|g| g.owner != hwnd.0 as isize) {
                    *cell = None; // the overlay window was made again
                }
                if cell.is_none() {
                    if rect.is_none() {
                        return;
                    }
                    match create(hwnd) {
                        Ok(g) => *cell = Some(g),
                        Err(e) => {
                            FAILED.store(true, Ordering::Relaxed);
                            log::warn!("overlay glass unavailable: {e}");
                            return;
                        }
                    }
                }
                if let Some(g) = cell.as_mut() {
                    if let Err(e) = apply(g, rect, scale) {
                        log::warn!("overlay glass: {e}");
                    }
                }
            });
        });
    }
}
