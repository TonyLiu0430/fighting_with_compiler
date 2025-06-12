use std::ffi::c_void;
use windows::core::{implement, Interface, ScopedInterface, HRESULT, PCSTR, PCWSTR};
use windows::Win32::Graphics::Direct3D::Fxc::{D3DCompileFromFile, D3DReadFileToBlob, D3DCOMPILE_DEBUG, D3DCOMPILE_ENABLE_STRICTNESS, D3DCOMPILE_SKIP_OPTIMIZATION};
use windows::Win32::Graphics::Direct3D::{ID3DBlob, ID3DInclude, ID3DInclude_Vtbl};
use windows_core::*;


pub fn create_shader_from_file(cso_file_name_in_out: PCWSTR, hlsl_file_name: PCWSTR, entry_point: PCSTR, shader_model: PCSTR) -> ID3DBlob {
    unsafe {
        let blob = D3DReadFileToBlob(cso_file_name_in_out);
        if blob.is_ok() {
            return blob.unwrap();
        }
    }

    let mut shader_flag = D3DCOMPILE_ENABLE_STRICTNESS;

    // DEBUG
    shader_flag |= D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION;

    let mut blob: Option<ID3DBlob> = None;
    unsafe {
        let include_flag = ID3DInclude::from_raw(1 as *mut c_void);
        let mut err_msg: Option<ID3DBlob> = None;
        let res = D3DCompileFromFile(hlsl_file_name, None, &include_flag, entry_point, shader_model, shader_flag, 0, &mut blob, Some(&mut err_msg));
        if res.is_err() {
            panic!("Compile failed with error: {}", res.unwrap_err());
        }
    }
    return blob.unwrap();
}