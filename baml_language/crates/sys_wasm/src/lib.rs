//! Browser implementations of the BAML `SysOps` table.
//!
//! The implementation modules are shared with `bridge_wasm`; this crate keeps
//! SDK consumers from linking the playground/LSP runtime and its wasm-bindgen
//! exports merely to obtain browser IO.

use std::sync::Arc;

use bex_events::run::{HostCallId, InMemoryRunStore};
use js_sys::Function;
use wasm_bindgen::prelude::*;

#[path = "../../bridge_wasm/src/host_value.rs"]
mod host_value;
#[path = "../../bridge_wasm/src/registry.rs"]
mod registry;
#[path = "../../bridge_wasm/src/send_wrapper.rs"]
mod send_wrapper;
#[path = "../../bridge_wasm/src/wasm_env.rs"]
mod wasm_env;
#[path = "../../bridge_wasm/src/wasm_http.rs"]
mod wasm_http;
#[path = "../../bridge_wasm/src/wasm_io.rs"]
mod wasm_io;
#[path = "../../bridge_wasm/src/wasm_sys.rs"]
mod wasm_sys;
#[path = "../../bridge_wasm/src/wasm_time.rs"]
mod wasm_time;

pub use host_value::{
    complete_host_call, mint_host_value_key, register_host_callable,
    register_host_value_release_callback, release_host_callable,
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "BrowserCallbacks")]
    pub type BrowserCallbacks;

    #[wasm_bindgen(method, getter, structural)]
    fn fetch(this: &BrowserCallbacks) -> Function;

    #[wasm_bindgen(method, getter, structural, js_name = "env")]
    fn env(this: &BrowserCallbacks) -> Function;

    #[wasm_bindgen(method, getter, structural, js_name = "input")]
    fn input(this: &BrowserCallbacks) -> Function;

    #[wasm_bindgen(method, getter, structural, js_name = "exec")]
    fn exec(this: &BrowserCallbacks) -> Function;

    #[wasm_bindgen(method, getter, structural, js_name = "shell")]
    fn shell(this: &BrowserCallbacks) -> Function;

    #[wasm_bindgen(method, getter, structural, js_name = "host_dispatch")]
    fn host_dispatch(this: &BrowserCallbacks) -> Function;
}

/// Build a browser `SysOps` table with the same implementations used by the
/// playground WASM runtime, excluding its separately supplied VFS/glob layer.
pub fn build(callbacks: &BrowserCallbacks) -> sys_ops::SysOps {
    let run_store = Arc::new(InMemoryRunStore::default());
    let notification = send_wrapper::SendWrapper::new(Function::new_no_args(""));

    sys_ops::SysOpsBuilder::new()
        .with_http_instance(Arc::new(wasm_http::WasmHttp::new(
            callbacks.fetch(),
            run_store.clone(),
            notification.clone(),
        )))
        .with_env_instance(Arc::new(wasm_env::WasmEnv::new(
            callbacks.env(),
            run_store.clone(),
            notification.clone(),
        )))
        .with_io_instance(Arc::new(wasm_io::WasmIo::new(
            callbacks.input(),
            run_store,
            notification,
        )))
        .with_sys_instance(Arc::new(wasm_sys::WasmSys::new(
            callbacks.exec(),
            callbacks.shell(),
        )))
        .with_time_instance(Arc::new(wasm_time::WasmTime))
        .with_host_instance(Arc::new(host_value::WasmHost::new(
            callbacks.host_dispatch(),
            true,
        )))
        .build()
}

pub(crate) fn wasm_host_call_id(call_id: sys_types::CallId) -> Option<HostCallId> {
    u32::try_from(call_id.0).ok().map(HostCallId::Wasm)
}

pub(crate) fn send_run_patch(
    _callback: &send_wrapper::SendWrapper<Function>,
    _patch: &bex_events::run::RunPatch,
) {
}
