#![allow(non_snake_case)]

use std::collections::HashMap;
use std::ffi::c_void;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock, RwLock, Weak};
use windows::core::PCWSTR;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use widestring::{u16str, U16Str};
use windows::Win32::Foundation;
use windows::Win32::Graphics::Gdi::UpdateWindow;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use once_cell::sync::OnceCell;
use windows_core::w;
use crate::d3d11::D3d11Renderer;

#[derive(Debug, Copy, Clone)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Copy, Clone)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

pub struct EventHandler {
    pub msg: u32,
    pub handler: Box<dyn FnMut(WPARAM, LPARAM)>
}

impl EventHandler {
    pub fn new(msg: u32, handler: Box<dyn FnMut(WPARAM, LPARAM)>) -> EventHandler {
        return  EventHandler {
            msg: msg,
            handler
        };
    }
}


pub struct Window {
    pub hwnd : HWND,
    callbacks: RwLock<Vec<EventHandler>>
}

impl Window {
    pub fn new(
        dw_ex_style: WINDOW_EX_STYLE,
        class_name: PCWSTR,
        window_name: PCWSTR,
        dw_style: WINDOW_STYLE,
        pos: Position,
        size: Size,
        parent: HWND,
        menu: HMENU,
        h_instance: HINSTANCE,
        lp_param: Option<*const ::core::ffi::c_void>,
    ) -> Result<Arc<Self>, String> {
        let hwnd = unsafe {
            CreateWindowExW(
                dw_ex_style,
                class_name,    // lpClassName
                window_name,   // lpWindowName
                dw_style,      // dwStyle
                pos.x,         // X
                pos.y,         // Y
                size.width,    // nWidth
                size.height,   // nHeight
                Option::from(parent),        // hWndParent
                Option::from(menu),          // hMenu
                Option::from(h_instance),    // hInstance
                lp_param,      // lpParam
            )
        };

        if hwnd.is_err() {
            return Err(hwnd.err().unwrap().to_string());
        }

        let hwnd = hwnd.ok().unwrap();

        let window_instance = Arc::new(Window { hwnd,  callbacks: vec![].into() });
        WndClass::get_instance().window_instances.write().unwrap().insert(hwnd.0, Arc::downgrade(&window_instance));

        Ok(window_instance)
    }

    pub fn show(&self, nCmdShow : SHOW_WINDOW_CMD) {
        unsafe {
            let _ = ShowWindow(self.hwnd, nCmdShow);
            let _ = UpdateWindow(self.hwnd);
        }
    }

    pub fn get_size(&self) -> Size {
        return unsafe {
            let mut rect = RECT::default();
            GetClientRect(self.hwnd, &mut rect).expect("TODO: panic message");
            Size {width : rect.right - rect.left, height : rect.bottom - rect.top}
        };
    }

    pub fn get_position(&self) -> Position {
        return unsafe {
            let mut rect = RECT::default();
            GetWindowRect(self.hwnd, &mut rect).expect("TODO: panic message");
            Position { x: rect.left, y: rect.top }
        }
    }

    pub fn add_handler(&self, handler: EventHandler) {
        let mut callbacks = self.callbacks.write().unwrap();
        callbacks.push(handler);
    }

    pub fn wnd_proc(&self,
                    hwnd: HWND,
                    msg: u32,
                    wparam: WPARAM,
                    lparam: LPARAM) -> LRESULT {
        for handler in self.callbacks.write().unwrap().iter_mut() {
            if handler.msg == msg {
                (handler.handler)(wparam, lparam);
            }
        }
        match msg {
            WM_PAINT => {
                return LRESULT(0);
            },
            WM_DESTROY => {
                unsafe {
                    PostQuitMessage(0);
                }
                // temp
                panic!("close window");
                return LRESULT(0);
            }
            _ => {
                return unsafe{ DefWindowProcW(hwnd, msg, wparam, lparam) };
            }
        }
    }
}

pub struct WindowBuilder {
    dw_ex_style : WINDOW_EX_STYLE,
    class_name : Option<PCWSTR>,
    window_name : Option<PCWSTR>,
    dw_style : WINDOW_STYLE,
    pos : Position,
    size : Size,
    parent : HWND,
    menu : HMENU,
    h_instance : Option<HINSTANCE>,
    lp_param : Option<*const ::core::ffi::c_void>,
}

impl<'a> WindowBuilder {
    /// 創建一個新的 `WindowBuilder` 實例，所有選項預設為 `None` 或預設值。
    pub fn new() -> Self {
        Self {
            dw_ex_style: WINDOW_EX_STYLE(0),
            class_name: None,
            window_name: None,
            dw_style: WS_OVERLAPPEDWINDOW,
            pos: { Position { x : CW_USEDEFAULT, y : CW_USEDEFAULT } } ,
            size: { Size { width : CW_USEDEFAULT, height : CW_USEDEFAULT } },
            parent: HWND::default(),
            menu: HMENU::default(),
            h_instance: None,
            lp_param: None,
        }
    }

    /// 設定擴展樣式。
    pub fn ex_style(mut self, style: WINDOW_EX_STYLE) -> Self {
        self.dw_ex_style = style;
        self
    }

    /// 設定視窗類別名稱。
    pub fn class_name(mut self, name: PCWSTR) -> Self {
        self.class_name = Some(name);
        self
    }

    /// 設定視窗名稱。
    pub fn window_name(mut self, name: PCWSTR) -> Self {
        self.window_name = Some(name);
        self
    }

    /// 設定視窗樣式。
    pub fn style(mut self, style: WINDOW_STYLE) -> Self {
        self.dw_style = style;
        self
    }

    /// 設定視窗位置。
    pub fn position(mut self, x: i32, y: i32) -> Self {
        self.pos = Position { x, y };
        self
    }

    /// 設定視窗尺寸。
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.size = Size { width, height };
        self
    }

    /// 設定父視窗。
    pub fn parent(mut self, parent_hwnd: HWND) -> Self {
        self.parent = parent_hwnd;
        self
    }

    /// 設定選單句柄。
    pub fn menu(mut self, h_menu: HMENU) -> Self {
        self.menu = h_menu;
        self
    }

    /// 設定應用程式實例句柄。
    pub fn hinstance(mut self, h_instance: HINSTANCE) -> Self {
        self.h_instance = Some(h_instance);
        self
    }

    /// 設定傳遞給視窗的創建參數。
    pub fn param(mut self, lp_param: *const ::core::ffi::c_void) -> Self {
        self.lp_param = Some(lp_param);
        self
    }

    /// 構建並創建 `Window` 實例。
    ///
    /// 這個方法會檢查所有必要參數是否已提供，並使用預設值來填充未提供的參數。
    pub fn build(self) -> Result<Arc<Window>, String> {
        let class_name = self.class_name.ok_or("Class name is required")?;
        let window_name = self.window_name.ok_or("Window name is required")?;
        let h_instance = self.h_instance.ok_or("HINSTANCE is required")?;

        // 設定預設值
        let dw_ex_style = self.dw_ex_style;
        let dw_style = self.dw_style;
        let pos = self.pos;
        let size = self.size;

        Window::new(
            dw_ex_style,
            class_name,
            window_name,
            dw_style,
            pos,
            size,
            self.parent,
            self.menu,
            h_instance,
            self.lp_param,
        )
    }
}





#[derive(Debug)]
pub struct WndClass {
    pub h_instance: HINSTANCE,
    window_instances: RwLock<HashMap<*const c_void, Weak<Window>>>
}


unsafe impl Send for WndClass {}
unsafe impl Sync for WndClass {}

static WND_CLASS: OnceLock<WndClass> = OnceLock::new();

impl WndClass {
    pub fn init(class_name : PCWSTR) {
        let h_instance : HINSTANCE = unsafe {
            GetModuleHandleW(PCWSTR::null())
        }.unwrap().into();
        let wndclass = WNDCLASSW {
            style: Default::default(),
            lpfnWndProc: Some(WndClass::wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: h_instance,
            hIcon: Default::default(),
            hCursor: Default::default(),
            hbrBackground: Default::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: class_name,
        };
        unsafe {
            RegisterClassW(&wndclass);
        }

        unsafe {
            let result = WndClass { h_instance, window_instances: RwLock::new(HashMap::new()) };
            WND_CLASS.set(result).unwrap();
        }
    }
    pub fn get_instance() -> &'static Self {
        WND_CLASS.get().unwrap()
    }
    pub fn msg_loop() {
        let mut msg = MSG::default();
        unsafe {
            while(GetMessageW(&mut msg, Option::from(HWND::default()), 0, 0)).into() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
    pub extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_NCCREATE => {
                // WM_NCCREATE 在 CreateWindowExW 內部發送，此時 WM_CREATE 尚未觸發
                // 如果在 Window::new 中立即插入了映射，則此處無需額外處理 `lp_param`
                // 而是直接讓後續訊息處理邏輯使用 HashMap 查詢
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
            WM_DESTROY => {
                let map = &WndClass::get_instance().window_instances.read().unwrap();
                if let Some(window_weak_arc) = map.get(&(hwnd.0 as *const c_void)) {
                    if let Some(window_arc) = window_weak_arc.upgrade() {
                        let result = window_arc.wnd_proc(hwnd, msg, wparam, lparam);
                        return result;
                    }
                }
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
            WM_NCDESTROY => {
                let map_to_remove = &mut WndClass::get_instance().window_instances.write().unwrap();
                map_to_remove.remove(&(hwnd.0 as *const c_void));

                // 如果是主視窗銷毀，則發送退出訊息
                // 這需要您有一些機制來識別主視窗
                // 這裡我們簡化處理，如果 HashMap 為空，就認為是最後一個視窗
                if map_to_remove.is_empty() {
                    println!("Last window destroyed, posting quit message.");
                    unsafe {
                        PostQuitMessage(0);
                    }
                }
                LRESULT(0)
            }
            _ => {
                let map = &WndClass::get_instance().window_instances.read().unwrap();
                if let Some(window_weak_arc) = map.get(&(hwnd.0 as *const c_void)) {
                    if let Some(window_arc) = window_weak_arc.upgrade() {
                        return window_arc.wnd_proc(hwnd, msg, wparam, lparam);
                    }
                }
                unsafe {
                    DefWindowProcW(hwnd, msg, wparam, lparam)
                }
            }
        }
    }
}

pub fn HIWORD(x: u32) -> u32 {
    (x >> 16) & 0xFFFF
}

pub fn LOWORD(x: u32) -> u32 {
    x & 0xFFFF
}