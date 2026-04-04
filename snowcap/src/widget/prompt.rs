//! Prompts wrap [`TextInput`], adding completion and history hook.
//!
//! [`TextInput`]: text_input::TextInput
use iced::{keyboard::key, widget::text_input};
use iced_wgpu::core::{
    self as iced_core, Element, Widget, keyboard, text,
    widget::{Tree, operation::TextInput, tree},
};

/// History request.
pub enum History {
    Search,
    Up,
    Down,
}

/// Completion request.
pub enum Completion {
    Previous,
    Next,
}

/// A wrapper around a [`TextInput`] that supports completion and history related messages.
pub struct Prompt<'a, Message, Theme = iced_core::Theme, Renderer = iced_renderer::Renderer>
where
    Theme: text_input::Catalog,
    Renderer: text::Renderer,
{
    text_input: text_input::TextInput<'a, Message, Theme, Renderer>,
    cursor: Option<usize>,
    on_history: Option<Box<dyn Fn(History, usize) -> Message + 'a>>,
    on_completion: Option<Box<dyn Fn(Completion, usize) -> Message + 'a>>,
}

impl<'a, Message, Theme, Renderer> Prompt<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Theme: text_input::Catalog,
    Renderer: text::Renderer,
{
    /// Creates a new [`Prompt`] with the given [`TextInput`].
    ///
    /// [`TextInput`]: text_input::TextInput
    pub fn new(text_input: text_input::TextInput<'a, Message, Theme, Renderer>) -> Self {
        let text_input = text_input.secure(false);

        Self {
            text_input,
            cursor: None,
            on_history: None,
            on_completion: None,
        }
    }

    /// Overrides the cursor location.
    pub fn cursor(self, cursor: usize) -> Self {
        Self {
            cursor: Some(cursor),
            ..self
        }
    }

    /// Sets the message that should be produced when some history search is requested.
    ///
    /// If this method is not called, history search will be disabled.
    pub fn on_history(self, on_history: impl Fn(History, usize) -> Message + 'a) -> Self {
        Self {
            on_history: Some(Box::new(on_history)),
            ..self
        }
    }

    /// Sets the message that should be produced when completion is requested.
    ///
    /// If this method is not called, completion will be disabled.
    pub fn on_completion(self, on_completion: impl Fn(Completion, usize) -> Message + 'a) -> Self {
        Self {
            on_completion: Some(Box::new(on_completion)),
            ..self
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Prompt<'_, Message, Theme, Renderer>
where
    Renderer: text::Renderer,
    Theme: text_input::Catalog,
    Message: Clone,
{
    fn tag(&self) -> tree::Tag {
        self.text_input.tag()
    }

    fn state(&self) -> tree::State {
        self.text_input.state()
    }

    fn diff(&self, tree: &mut Tree) {
        if let Some(idx) = self.cursor.as_ref() {
            let state = state::<Renderer>(tree);
            state.move_cursor_to(*idx);
        }

        self.text_input.diff(tree);
    }

    fn size(&self) -> iced::Size<iced::Length> {
        Widget::size(&self.text_input)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &iced_core::layout::Limits,
    ) -> iced_core::layout::Node {
        Widget::layout(&mut self.text_input, tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: iced_core::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced_core::widget::Operation,
    ) {
        Widget::operate(&mut self.text_input, tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: iced_core::Layout<'_>,
        cursor: iced_core::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced_core::Clipboard,
        shell: &mut iced_core::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) {
        Widget::update(
            &mut self.text_input,
            tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        update(self, tree, event, shell);
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced_core::renderer::Style,
        layout: iced_core::Layout<'_>,
        cursor: iced_core::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        Widget::draw(
            &self.text_input,
            tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: iced_core::Layout<'_>,
        cursor: iced_core::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> iced_core::mouse::Interaction {
        Widget::mouse_interaction(&self.text_input, tree, layout, cursor, viewport, renderer)
    }
}

impl<'a, Message, Theme, Renderer> From<text_input::TextInput<'a, Message, Theme, Renderer>>
    for Prompt<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: text_input::Catalog + 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(value: text_input::TextInput<'a, Message, Theme, Renderer>) -> Self {
        Self::new(value)
    }
}

impl<'a, Message, Theme, Renderer> From<Prompt<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: text_input::Catalog + 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(widget: Prompt<'a, Message, Theme, Renderer>) -> Self {
        Element::new(widget)
    }
}

fn state<Renderer: text::Renderer>(tree: &mut Tree) -> &mut text_input::State<Renderer::Paragraph> {
    tree.state
        .downcast_mut::<text_input::State<Renderer::Paragraph>>()
}

fn update<Message, Theme, Renderer>(
    widget: &mut Prompt<'_, Message, Theme, Renderer>,
    tree: &mut Tree,
    event: &iced::Event,
    shell: &mut iced_core::Shell<'_, Message>,
) where
    Message: Clone,
    Theme: text_input::Catalog,
    Renderer: text::Renderer,
{
    use iced::Event;

    let state = state::<Renderer>(tree);

    if !state.is_focused() {
        return;
    }

    let Event::Keyboard(keyboard::Event::KeyPressed {
        key,
        modified_key,
        physical_key,
        modifiers,
        repeat,
        ..
    }) = &event
    else {
        return;
    };

    if *repeat {
        return;
    }

    let value = text_input::Value::new(state.text());
    let cursor_index = match state.cursor().state(&value) {
        text_input::cursor::State::Index(idx) => idx,
        text_input::cursor::State::Selection { start, .. } => start,
    };

    match key.to_latin(*physical_key) {
        Some('r') if modifiers.command() => {
            if let Some(on_history) = widget.on_history.as_ref() {
                shell.publish(on_history(History::Search, cursor_index));
                shell.capture_event();
            }

            return;
        }
        _ => (),
    }

    match modified_key.as_ref() {
        keyboard::Key::Named(key::Named::Tab) => {
            if let Some(on_completion) = widget.on_completion.as_ref() {
                let direction = if modifiers.shift() {
                    Completion::Previous
                } else {
                    Completion::Next
                };

                shell.publish(on_completion(direction, cursor_index));
                shell.capture_event();
            }
        }
        keyboard::Key::Named(key::Named::ArrowUp) => {
            if let Some(on_history) = widget.on_history.as_ref() {
                shell.publish(on_history(History::Up, cursor_index));
                shell.capture_event();
            }
        }
        keyboard::Key::Named(key::Named::ArrowDown) => {
            if let Some(on_history) = widget.on_history.as_ref() {
                shell.publish(on_history(History::Down, cursor_index));
                shell.capture_event();
            }
        }
        _ => {}
    }
}
