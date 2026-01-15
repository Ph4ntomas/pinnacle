use std::sync::{Arc, Mutex};

use bitflags::bitflags;
use smithay_client_toolkit::{
    reexports::{
        client::{Dispatch, Proxy, event_created_child, protocol::wl_output::WlOutput},
        protocols_wlr::foreign_toplevel::v1::client::{
            zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
            zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
        },
    },
    registry::GlobalProxy,
};

use crate::state::State;

pub struct ZwlrForeignToplevelManagementState {
    _manager: GlobalProxy<ZwlrForeignToplevelManagerV1>,
    toplevels: Vec<ZwlrForeignToplevelHandleV1>,
}

#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct ForeignToplevelInfo {
    pub app_id: String,
    pub title: String,
    pub outputs: Vec<WlOutput>, // TODO: Replace by the output id or name ?
    pub state: ToplevelState,
    // TODO: What about parents ?
}

#[derive(Debug, Default)]
pub struct ForeignToplevelInner {
    current_info: Option<ForeignToplevelInfo>,
    pending_info: ForeignToplevelInfo,
}

#[derive(Debug, Default, Clone)]
pub struct ForeignToplevelData(Arc<Mutex<ForeignToplevelInner>>);

#[derive(Debug, Clone)]
pub enum ZwlrForeignToplevelEvent {
    Added(ZwlrForeignToplevelHandleV1),
    Closed(ZwlrForeignToplevelHandleV1),
    Changed(ZwlrForeignToplevelHandleV1),
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ToplevelState: u32 {
        const None = 0;
        const Maximized = 1;
        const Minimized = 2;
        const Activated = 4;
        const Fullscreen = 8;
    }
}

impl ZwlrForeignToplevelManagementState {
    pub fn new(
        globals: &smithay_client_toolkit::reexports::client::globals::GlobalList,
        qh: &smithay_client_toolkit::reexports::client::QueueHandle<State>,
    ) -> Self {
        let _manager = GlobalProxy::from(globals.bind(qh, 1..=3, ()));

        Self {
            _manager,
            toplevels: Vec::new(),
        }
    }

    pub fn toplevels(&self) -> &[ZwlrForeignToplevelHandleV1] {
        &self.toplevels
    }

    pub fn info(&self, toplevel: &ZwlrForeignToplevelHandleV1) -> Option<ForeignToplevelInfo> {
        toplevel
            .data::<ForeignToplevelData>()?
            .0
            .lock()
            .unwrap()
            .current_info
            .clone()
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrForeignToplevelManagerV1,
        event: <ZwlrForeignToplevelManagerV1 as smithay_client_toolkit::reexports::client::Proxy>::Event,
        _data: &(),
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qhandle: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
    ) {
        match event {
            zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel: _ } => {}
            zwlr_foreign_toplevel_manager_v1::Event::Finished => (),
            _ => unreachable!(),
        }

        // TODO:
    }

    event_created_child!(State, ZwlrForeignToplevelManagerV1, [
        zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, Default::default())
    ]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ForeignToplevelData> for State {
    fn event(
        state: &mut Self,
        proxy: &ZwlrForeignToplevelHandleV1,
        event: <ZwlrForeignToplevelHandleV1 as smithay_client_toolkit::reexports::client::Proxy>::Event,
        data: &ForeignToplevelData,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qhandle: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
    ) {
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                state.zwlr_toplevel_closed(proxy.clone());
                state
                    .zwlr_foreign_toplevel_mgmt_state
                    .toplevels
                    .retain(|t| t != proxy);
                proxy.destroy();
            }
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                data.0.lock().unwrap().pending_info.title = title;
            }
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                data.0.lock().unwrap().pending_info.app_id = app_id;
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputEnter { output } => {
                data.0.lock().unwrap().pending_info.outputs.push(output);
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputLeave { output } => {
                data.0
                    .lock()
                    .unwrap()
                    .pending_info
                    .outputs
                    .retain(|o| o != &output);
            }
            zwlr_foreign_toplevel_handle_v1::Event::State { state: flags } => {
                data.0.lock().unwrap().pending_info.state = flags.into();
            }
            zwlr_foreign_toplevel_handle_v1::Event::Parent { parent: _ } => (),
            zwlr_foreign_toplevel_handle_v1::Event::Done => {
                let mut inner = data.0.lock().unwrap();
                let new_toplevel = inner.current_info.is_none();
                inner.current_info = Some(inner.pending_info.clone());
                std::mem::drop(inner);

                if new_toplevel {
                    state
                        .zwlr_foreign_toplevel_mgmt_state
                        .toplevels
                        .push(proxy.clone());
                    state.new_zwlr_toplevel(proxy.clone());
                } else {
                    state.zwlr_toplevel_updated(proxy.clone());
                }
            }
            _ => (),
        }
    }
}

impl State {
    // TODO: Log errors ?
    pub fn new_zwlr_toplevel(&self, handle: ZwlrForeignToplevelHandleV1) {
        let _ = self
            .zwlr_foreign_toplevel_sender
            .send(ZwlrForeignToplevelEvent::Added(handle));
    }

    pub fn zwlr_toplevel_updated(&self, handle: ZwlrForeignToplevelHandleV1) {
        let _ = self
            .zwlr_foreign_toplevel_sender
            .send(ZwlrForeignToplevelEvent::Changed(handle));
    }

    pub fn zwlr_toplevel_closed(&self, handle: ZwlrForeignToplevelHandleV1) {
        let _ = self
            .zwlr_foreign_toplevel_sender
            .send(ZwlrForeignToplevelEvent::Closed(handle));
    }
}

impl ForeignToplevelData {
    pub fn with_info<F, Ret>(&self, processor: F) -> Option<Ret>
    where
        F: FnOnce(&ForeignToplevelInfo) -> Ret,
    {
        self.0.lock().ok()?.current_info.as_ref().map(processor)
    }
}

impl From<Vec<u8>> for ToplevelState {
    fn from(value: Vec<u8>) -> Self {
        value.iter().fold(Self::None, |acc, val| {
            let flag = match &val {
                0 => Self::Maximized,
                1 => Self::Minimized,
                2 => Self::Activated,
                3 => Self::Fullscreen,
                _ => Self::None,
            };

            acc | flag
        })
    }
}

impl Default for ToplevelState {
    fn default() -> Self {
        Self::None
    }
}
