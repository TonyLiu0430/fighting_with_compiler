use std::{mem, slice};
use directx_math::{XMFLOAT3, XMFLOAT4};
use windows::core::{s, w, Interface, BOOL};
use windows::Win32::Foundation::{HMODULE, HWND, SIZE};
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Direct3D::{ID3DBlob, ID3DInclude, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST, D3D_DRIVER_TYPE, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1};
use windows::Win32::Graphics::Dxgi::{Common, IDXGIAdapter, IDXGIDevice, IDXGIFactory2, IDXGISwapChain, IDXGISwapChain1, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FLAG, DXGI_SWAP_CHAIN_FULLSCREEN_DESC, DXGI_SWAP_EFFECT_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_D24_UNORM_S8_UINT, DXGI_FORMAT_R32G32B32A32_FLOAT, DXGI_FORMAT_R32G32B32_FLOAT, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_SCALING_UNSPECIFIED, DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED};
use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;
use windows::Win32::Graphics::Direct3D::Fxc;
use windows::Win32::Graphics::Direct3D::Fxc::D3DCompileFromFile;
use crate::d3dutil::create_shader_from_file;
use crate::window::{Position, Size, Window};

pub struct D3d11Renderer{
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    swap_chain: IDXGISwapChain1,
    render_target_view: Option<ID3D11RenderTargetView>,
    depth_stencil_view: Option<ID3D11DepthStencilView>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct VertexPosColor {
    position: XMFLOAT3,
    color: XMFLOAT4,
}

impl D3d11Renderer {
    pub fn new(d3d_driver_type : D3D_DRIVER_TYPE, window: &Window) -> D3d11Renderer {
        let (device, context) = Self::create_device_context(d3d_driver_type);
        let pos = window.get_position();
        let size = window.get_size();
        let swap_chain = Self::create_swap_chain(&device, window.hwnd, size);
        let (render_target_view, depth_stencil_view) = Self::create_views(&device, &swap_chain, size);
        Self::bind_render_target(&context, &render_target_view, &depth_stencil_view);
        Self::set_viewport(&context, pos, size);
        return Self {
            device,
            context,
            swap_chain,
            render_target_view : Some(render_target_view),
            depth_stencil_view : Some(depth_stencil_view)
        };
    }

    fn create_device_context(d3d_driver_type : D3D_DRIVER_TYPE) -> (ID3D11Device, ID3D11DeviceContext) {
        let mut flag = D3D11_CREATE_DEVICE_FLAG::default();
        flag |= D3D11_CREATE_DEVICE_DEBUG;
        let mut feature_level = D3D_FEATURE_LEVEL_11_1;
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        unsafe {
            D3D11CreateDevice(
                None,
                d3d_driver_type,
                HMODULE::default(),
                flag,
                Some(&[D3D_FEATURE_LEVEL_11_1]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                Some(&mut feature_level),
                Some(&mut context),
            ).unwrap()
        }

        let device = device.unwrap();
        let context = context.unwrap();
        (device, context)
    }

    fn create_swap_chain(device: &ID3D11Device, hwnd : HWND, size : Size) -> IDXGISwapChain1 {
        let dxgi_device = device.clone().cast::<IDXGIDevice>().unwrap();
        let adapter = unsafe {
            dxgi_device.GetAdapter().unwrap()
        };

        let factory = unsafe {
            adapter.GetParent::<IDXGIFactory2>().unwrap()
        };

        let swap_chain_desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: size.width as u32, //binding window size
            Height: size.height as u32,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            Stereo: Default::default(),
            SampleDesc: Common::DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 1,
            Scaling: Default::default(),
            SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
            AlphaMode: Default::default(),
            Flags: 0,
        };

        let fullscreen_desc = DXGI_SWAP_CHAIN_FULLSCREEN_DESC{
            RefreshRate: Common::DXGI_RATIONAL {
                Numerator: 60,
                Denominator: 1
            },
            ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
            Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
            Windowed: BOOL::from(true),
        };

        let swap_chain = unsafe {
            factory.CreateSwapChainForHwnd(&*device, hwnd, &swap_chain_desc, Some(&fullscreen_desc), None)
        }.unwrap();

        swap_chain
    }

    // swap chain 要先初始化完成
    fn create_views(device : &ID3D11Device, swap_chain : &IDXGISwapChain1, size: Size) -> (ID3D11RenderTargetView, ID3D11DepthStencilView) {
        let back_buffer = unsafe {
            swap_chain.GetBuffer::<ID3D11Texture2D>(0).unwrap()
        };
        let mut render_target_view: Option<ID3D11RenderTargetView> = None;
        unsafe {
            device.CreateRenderTargetView(&back_buffer, None, Some(&mut render_target_view)).unwrap();
        }

        let render_target_view = render_target_view.unwrap();

        let depth_stencil_desc = D3D11_TEXTURE2D_DESC {
            Width: size.width as u32,
            Height: size.height as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_D24_UNORM_S8_UINT,
            SampleDesc: Common::DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_DEPTH_STENCIL.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };

        let mut depth_stencil_buffer: Option<ID3D11Texture2D> = None;

        unsafe {
            device.CreateTexture2D(&depth_stencil_desc, None, Some(&mut depth_stencil_buffer)).unwrap()
        }

        let depth_stencil_buffer = depth_stencil_buffer.unwrap();

        let mut depth_stencil_view: Option<ID3D11DepthStencilView> = None;
        unsafe {
            device.CreateDepthStencilView(&depth_stencil_buffer, None, Some(&mut depth_stencil_view)).unwrap();
        }

        let depth_stencil_view = depth_stencil_view.unwrap();

        (render_target_view, depth_stencil_view)
    }

    fn bind_render_target(context: &ID3D11DeviceContext, render_target_view: &ID3D11RenderTargetView, depth_stencil_view: &ID3D11DepthStencilView) {
        unsafe {
            let target_views = [Some(render_target_view.clone())];
            context.OMSetRenderTargets(Some(&target_views), depth_stencil_view)
        }
    }

    fn clear_render_target(context: &ID3D11DeviceContext) {
        unsafe {
            context.OMSetRenderTargets(None, None)
        }
    }

    fn set_viewport(context: &ID3D11DeviceContext, pos : Position, size: Size) {
        let viewport = D3D11_VIEWPORT {
            TopLeftX: pos.x as f32,
            TopLeftY: pos.y as f32,
            Width: size.width as f32,
            Height: size.height as f32,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };

        unsafe {
            context.RSSetViewports(Some(&[viewport]));
        }
    }

    
    pub fn fill_blue(&self) {
        // let blue: [f32; 4] = [0f32, 0f32, 1f32, 0f32];
        // unsafe {
        //     self.context.ClearRenderTargetView(&self.render_target_view, &blue);
        //     let flag = D3D11_CLEAR_DEPTH | D3D11_CLEAR_STENCIL;
        //     self.context.ClearDepthStencilView(&self.depth_stencil_view, flag.0, 1f32, 0);
        // }
    }
    pub fn present(&self) {
        unsafe {
            let _ = self.swap_chain.Present(0, windows::Win32::Graphics::Dxgi::DXGI_PRESENT(0));
        }
    }

    fn load_hlsl(&self) -> (ID3D11InputLayout, ID3D11VertexShader, ID3D11PixelShader) {
        let input_layout = [
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: s!("POSITION"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32B32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 0,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: s!("COLOR"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 12,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
        ];
        let mut vertex_layout: Option<ID3D11InputLayout> = None;
        let mut vertex_shader: Option<ID3D11VertexShader> = None;
        unsafe {
            // 頂點著色器
            let vs_blob = create_shader_from_file(w!("hlsl/triangle_vs.cso"), w!("hlsl/triangle_vs.hlsl"), s!("VS"), s!("vs_5_0"));

            let vs_buffer = slice::from_raw_parts(vs_blob.GetBufferPointer() as *mut u8, vs_blob.GetBufferSize());

            self.device.CreateVertexShader(vs_buffer, None, Some(&mut vertex_shader)).expect("TODO: panic message");

            self.device.CreateInputLayout(&input_layout, vs_buffer, Some(&mut vertex_layout)).expect("TODO: panic message");
        }
        let mut pixel_shader: Option<ID3D11PixelShader> = None;
        unsafe {
            // 像素著色器
            let ps_blob = create_shader_from_file(w!("hlsl/triangle_ps.cso"), w!("hlsl/triangle_ps.hlsl"), s!("PS"), s!("ps_5_0"));

            let ps_buffer = slice::from_raw_parts(ps_blob.GetBufferPointer() as *mut u8, ps_blob.GetBufferSize());
            self.device.CreatePixelShader(ps_buffer, None, Some(&mut pixel_shader)).expect("TODO");
        }

        return (vertex_layout.unwrap(), vertex_shader.unwrap(), pixel_shader.unwrap())
    }

    pub fn render(&self) {

        let vertices = [
            VertexPosColor {
                position: XMFLOAT3 {
                    x: 0.0,
                    y: 0.5,
                    z: 0.5,
                },
                color: XMFLOAT4 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                    w: 1.0,
                }
            },
            VertexPosColor {
                position: XMFLOAT3 {
                    x: 0.0,
                    y: -0.5,
                    z: 0.5,
                },
                color: XMFLOAT4 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                    w: 1.0,
                }
            },
            VertexPosColor {
                position: XMFLOAT3 {
                    x: -0.5,
                    y: -0.5,
                    z: 0.5,
                },
                color: XMFLOAT4 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                    w: 1.0,
                }
            },
        ];

        let vbd = D3D11_BUFFER_DESC{
            ByteWidth: size_of_val(&vertices) as u32,
            Usage: D3D11_USAGE_IMMUTABLE,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: 0,
        };
        let init_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: vertices.as_ptr() as _,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };

        let mut buffer: Option<ID3D11Buffer> = None;
        unsafe {
            self.device.CreateBuffer(&vbd, Some(&init_data), Some(&mut buffer)).expect("REASON")
        }
        let buffer = buffer.unwrap();

        let stride = size_of::<VertexPosColor>() as u32;
        let offset = 0_u32;

        let (vertex_layout, vertex_shader, pixel_shader) = self.load_hlsl();
        unsafe {
            self.context.IASetVertexBuffers(0, 1, Some(&Some(buffer)), Some(&stride), Some(&offset));
            self.context.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            self.context.IASetInputLayout(&vertex_layout);
            self.context.VSSetShader(&vertex_shader, None);
            self.context.PSSetShader(&pixel_shader, None);
        }
    }

    pub fn draw_scene(&self) {
        let black = [0f32, 0f32, 0f32, 1f32];
        unsafe {
            // fill with black
            self.context.ClearRenderTargetView(&self.render_target_view.clone().unwrap(), &black);
            self.context.ClearDepthStencilView(&self.depth_stencil_view.clone().unwrap(), (D3D11_CLEAR_DEPTH | D3D11_CLEAR_STENCIL).0, 1.0, 0);

            // draw triangle
            self.context.Draw(3, 0);
            let _ = self.swap_chain.Present(0, windows::Win32::Graphics::Dxgi::DXGI_PRESENT(0));
        }
    }

    pub fn on_resize(&mut self, pos: Position, size: Size) {

        self.render_target_view = None;
        self.depth_stencil_view = None;

        unsafe {
            self.swap_chain.ResizeBuffers(1, size.width as u32, size.height as u32, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SWAP_CHAIN_FLAG(0)).expect("Resize failed");
        }

        let (render_target_view, depth_stencil_view) = Self::create_views(&self.device, &self.swap_chain, size);
        self.render_target_view = Some(render_target_view);
        self.depth_stencil_view = Some(depth_stencil_view);

        Self::bind_render_target(&self.context, &self.render_target_view.clone().unwrap(), &self.depth_stencil_view.clone().unwrap());
        Self::set_viewport(&self.context, pos, size);
    }
}


