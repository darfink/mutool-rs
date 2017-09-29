#![recursion_limit="128"]
#![feature(
    const_fn,
    drop_types_in_const,
    conservative_impl_trait,
    abi_thiscall,
)]

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate detour;
#[macro_use] extern crate matches;

#[macro_use] extern crate pest_derive;
extern crate pest;

#[macro_use] extern crate serde_json;
#[macro_use] extern crate serde_derive;
extern crate serde;

extern crate musdk as mu;
extern crate muonline_packet;
extern crate strsim;
extern crate knock;
extern crate toml;
extern crate tap;
extern crate num_traits;

#[cfg(windows)] extern crate kernel32;
#[cfg(windows)] extern crate user32;
#[cfg(windows)] extern crate winapi;

macro_rules! try_opt {
    ($expr:expr) => (match $expr {
        ::std::option::Option::Some(val) => val,
        ::std::option::Option::None => return,
    })
}

use main::MuTool;
use filter::ItemFilter;

mod ext;
mod filter;
mod main;
mod util;

static mut TOOL: Option<MuTool> = None;

#[no_mangle]
#[allow(non_snake_case)]
#[cfg(windows)]
pub unsafe extern "system" fn DllMain(
        _module: winapi::HINSTANCE,
        reason: winapi::DWORD,
        _reserved: winapi::LPVOID) -> winapi::BOOL {
    const DLL_PROCESS_ATTACH: winapi::DWORD = 1;
    //const DLL_PROCESS_DETACH: winapi::DWORD = 0;

    match reason {
        DLL_PROCESS_ATTACH => {
            assert!(kernel32::AllocConsole() != 0, "creating console");
            println!("[Main] Initializing MuTool...");
            match MuTool::new() {
                Err(error) => eprintln!("[Main:Error] Failed to initialize: {}", error),
                Ok(tool) => {
                    println!("[Main] Initialized MuTool");
                    TOOL = Some(tool);
                },
            }
        },
        _ => ()
    }

    return winapi::TRUE;
}

#[cfg(unix)]
pub unsafe extern "C" fn dll_main() {
    MuTool::new().unwrap();
}
