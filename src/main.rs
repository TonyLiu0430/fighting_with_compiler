mod window;
mod d3d11;
mod d3dutil;

use std::sync::{Arc, RwLock};
use windows::core::{s, w};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;
use window::*;
use widestring::{u16str, U16Str};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use crate::d3d11::D3d11Renderer;

fn main() {
    WndClass::init(w!("test string"));
    let global_wndclass = WndClass::get_instance();
    let mut window = WindowBuilder::new()
        .window_name(w!("test window"))
        .class_name(w!("test string"))
        .hinstance(global_wndclass.h_instance)
    .build().unwrap();

    let d3d11 = D3d11Renderer::new(D3D_DRIVER_TYPE_HARDWARE, &window);
    
    let pos = window.get_position();
    
    let d3d11 = Arc::new(RwLock::new(d3d11));

    window.show(SHOW_WINDOW_CMD(1));
    d3d11.read().unwrap().render();
    d3d11.read().unwrap().draw_scene();
    
    let d3d11_clone = d3d11.clone();
    window.add_handler(EventHandler::new(WM_SIZE, Box::new(move |wparam: WPARAM, lparam: LPARAM| {
        let width = LOWORD(lparam.0 as u32);
        let height = LOWORD(lparam.0 as u32);
        d3d11_clone.write().unwrap().on_resize(pos, Size{width: width as i32, height: height as i32});
        d3d11_clone.read().unwrap().render();
        d3d11_clone.read().unwrap().draw_scene();
    })));
    WndClass::msg_loop();
}
