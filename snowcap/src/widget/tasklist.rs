//! A widget to represent a list of open window.

use anyhow::Context;
use iced::{Length, Size};
use iced_wgpu::core::{
    Element, Widget, layout, renderer,
    widget::{Tree, tree},
};
use smithay_client_toolkit::reexports::{
    client::Proxy,
    protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
};

use crate::{
    handlers::foreign_toplevel_management::{
        ForeignToplevelData, ForeignToplevelInfo, ToplevelState,
    },
    widget::output::OutputState,
};

pub mod operation {
    use iced_wgpu::core::widget::Operation;
    use smithay_client_toolkit::reexports::protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1;

    pub fn new_toplevel(handle: ZwlrForeignToplevelHandleV1) -> impl Operation {
        struct AddToplevel {
            handle: ZwlrForeignToplevelHandleV1,
        }

        impl Operation for AddToplevel {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
                operate(self);
            }

            fn custom(
                &mut self,
                _id: Option<&iced::widget::Id>,
                _bounds: iced::Rectangle,
                state: &mut dyn std::any::Any,
            ) {
                let Some(_state) = state.downcast_mut::<super::State>() else {
                    return;
                };

                //state.add_toplevel(self.handle.clone());
            }
        }

        AddToplevel { handle }
    }

    pub fn update_toplevel(handle: ZwlrForeignToplevelHandleV1) -> impl Operation {
        struct UpdateToplevel {
            handle: ZwlrForeignToplevelHandleV1,
        }

        impl Operation for UpdateToplevel {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
                operate(self);
            }

            fn custom(
                &mut self,
                _id: Option<&iced::widget::Id>,
                _bounds: iced::Rectangle,
                state: &mut dyn std::any::Any,
            ) {
                let Some(_state) = state.downcast_mut::<super::State>() else {
                    return;
                };

                //state.update_toplevel(self.handle.clone());
            }
        }

        UpdateToplevel { handle }
    }

    pub fn remove_toplevel(handle: ZwlrForeignToplevelHandleV1) -> impl Operation {
        struct RemoveToplevel {
            handle: ZwlrForeignToplevelHandleV1,
        }

        impl Operation for RemoveToplevel {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
                operate(self);
            }

            fn custom(
                &mut self,
                _id: Option<&iced::widget::Id>,
                _bounds: iced::Rectangle,
                state: &mut dyn std::any::Any,
            ) {
                let Some(_state) = state.downcast_mut::<super::State>() else {
                    return;
                };

                //state.remove_toplevel(self.unique_id);
            }
        }

        RemoveToplevel { handle }
    }
}

#[derive(Debug, Clone)]
pub struct TaskState {
    pub title: String,
    pub app_id: String,
    pub state: ToplevelState,
}

#[derive(Debug, Clone)]
pub enum TaskListEvent {
    ToplevelEnter(u64, TaskState),
    ToplevelUpdate(u64, TaskState),
    ToplevelLeave(u64),
}

/// Emits events on window changes.
pub struct TaskList<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    on_enter: Option<Box<dyn Fn(u64, TaskState) -> Message + 'a>>,
    on_update: Option<Box<dyn Fn(u64, TaskState) -> Message + 'a>>,
    on_leave: Option<Box<dyn Fn(u64) -> Message + 'a>>,
    all_output: bool,
    //toplevel_manager: &'a ZwlrForeignToplevelManagementState,
}

/// Local state of the [`TaskList`].
#[derive(Default)]
pub struct State {
    output_state: OutputState,
    //toplevel_list: Vec<(u64, Weak<ZwlrForeignToplevelHandleV1>)>,

    //pending_add: Vec<ZwlrForeignToplevelHandleV1>,
    //pending_remove: Vec<ZwlrForeignToplevelHandleV1>,
    //pending_change: Vec<(u64, ZwlrForeignToplevelState, ZwlrForeignToplevelChanges)>,
    initial_state_sent: bool, // FIXME: This feels like a hack, but the alternative I can think of
                              // requires the client to fetch the initial list.
}

impl<'a, Message, Theme, Renderer> TaskList<'a, Message, Theme, Renderer> {
    // TODO: messages handling.
}

impl<'a, Message, Theme, Renderer> TaskList<'a, Message, Theme, Renderer> {
    /// Creates a [`TaskList`] with the given content.
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        TaskList {
            content: content.into(),
            on_enter: None,
            on_update: None,
            on_leave: None,
            all_output: false,
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for TaskList<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
    Message: Clone,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: layout::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced_wgpu::core::widget::Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();

        operation.custom(None, layout.bounds(), &mut state.output_state);
        operation.custom(None, layout.bounds(), state);

        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout.children().next().unwrap(),
                renderer,
                operation,
            );
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: layout::Layout<'_>,
        cursor: iced_wgpu::core::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced_wgpu::core::Clipboard,
        shell: &mut iced_wgpu::core::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let _state = tree.state.downcast_mut::<State>();

        //for (id, state) in state.pending_add.drain(..) {
        //}

        //for (id, state, changes) in state.pending_change.drain(..) {

        //}

        //for (id) in state.pending_remove.drain(..) {

        //}

        // TODO: Initial setup.

        //if shell.is_event_captured() {
        //return;
        //}

        // update(self, tree, event, layout, cursor, shell);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: layout::Layout<'_>,
        cursor: iced_wgpu::core::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> iced_wgpu::core::mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &iced_wgpu::core::renderer::Style,
        layout: layout::Layout<'_>,
        cursor: iced_wgpu::core::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            layout,
            cursor,
            viewport,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: layout::Layout<'b>,
        renderer: &Renderer,
        viewport: &iced::Rectangle,
        translation: iced::Vector,
    ) -> Option<iced_wgpu::core::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<TaskList<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(value: TaskList<'a, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}

impl TryFrom<ZwlrForeignToplevelHandleV1> for TaskState {
    type Error = anyhow::Error;

    fn try_from(value: ZwlrForeignToplevelHandleV1) -> anyhow::Result<Self> {
        let data = value
            .data::<ForeignToplevelData>()
            .context("Proxy has no associated data")?;

        data.with_info(|info| {
            let ForeignToplevelInfo {
                app_id,
                title,
                outputs: _,
                state,
            } = info.clone();
            Self {
                app_id,
                title,
                state,
            }
        })
        .context("Could not get TaskState from proxy.")
    }
}
