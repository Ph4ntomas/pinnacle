use snowcap_api_defs::snowcap::widget;

use super::{Widget, WidgetDef, WidgetId};

#[derive(Clone)]
pub struct MouseArea<Msg> {
    pub child: WidgetDef<Msg>,
    pub interaction: Option<Interaction>,
    pub unique_id: Option<String>,
    pub(crate) widget_id: Option<WidgetId>,
    pub(crate) on_press: Option<Msg>,
    pub(crate) on_release: Option<Msg>,
    pub(crate) on_double_click: Option<Msg>,
    pub(crate) on_right_press: Option<Msg>,
    pub(crate) on_right_release: Option<Msg>,
    pub(crate) on_middle_press: Option<Msg>,
    pub(crate) on_middle_release: Option<Msg>,
    pub(crate) on_enter: Option<Msg>,
    //pub(crate) on_scroll: Option<Box<dyn Fn(ScrollDelta) -> Msg>>,
    pub(crate) on_exit: Option<Msg>,
    //pub(crate) on_move: Option<Box<dyn Fn(Point) -> Msg>>,
}

impl<Msg: std::fmt::Debug> std::fmt::Debug for MouseArea<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MouseArea")
            .field("child", &self.child)
            .field("interaction", &self.interaction)
            .field("unique_id", &self.unique_id)
            .finish()
    }
}

impl<Msg: PartialEq> PartialEq for MouseArea<Msg> {
    fn eq(&self, other: &Self) -> bool {
        self.child == other.child
    }
}

impl<Msg> MouseArea<Msg> {
    pub fn new(child: impl Into<WidgetDef<Msg>>) -> Self {
        Self {
            child: child.into(),
            interaction: None,
            widget_id: None,
            unique_id: None,
            on_press: None,
            on_release: None,
            on_double_click: None,
            on_right_press: None,
            on_right_release: None,
            on_middle_press: None,
            on_middle_release: None,
            on_enter: None,
            //on_scroll: None,
            on_exit: None,
            //on_move: None,
        }
    }

    pub fn interaction(self, interaction: Interaction) -> Self {
        Self {
            interaction: Some(interaction),
            ..self
        }
    }

    pub fn unique_id(self, unique_id: impl Into<String>) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            unique_id: Some(unique_id.into()),
            ..self
        }
    }

    pub fn on_press(self, on_press: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            on_press: Some(on_press),
            ..self
        }
    }

    pub fn on_release(self, on_release: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            on_release: Some(on_release),
            ..self
        }
    }

    pub fn on_double_click(self, on_double_click: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            on_double_click: Some(on_double_click),
            ..self
        }
    }

    pub fn on_right_press(self, on_right_press: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            on_right_press: Some(on_right_press),
            ..self
        }
    }

    pub fn on_right_release(self, on_right_release: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            on_right_release: Some(on_right_release),
            ..self
        }
    }

    pub fn on_middle_press(self, on_middle_press: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            on_middle_press: Some(on_middle_press),
            ..self
        }
    }

    pub fn on_middle_release(self, on_middle_release: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            on_middle_release: Some(on_middle_release),
            ..self
        }
    }

    pub fn on_enter(self, on_enter: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            on_enter: Some(on_enter),
            ..self
        }
    }

    //pub fn on_scroll(self, on_scroll: _) -> Self {
        //unimplemented!()
    //}

    pub fn on_exit(self, on_exit: Msg) -> Self {
        Self {
            widget_id: self.widget_id.or_else(|| Some(WidgetId::next())),
            on_exit: Some(on_exit),
            ..self
        }
    }

    //pub fn on_move(self, on_move: _) -> Self {
        //unimplemented!()
    //}
}

impl<Msg> From<MouseArea<Msg>> for Widget<Msg> {
    fn from(value: MouseArea<Msg>) -> Self {
        Widget::MouseArea(Box::new(value))
    }
}

impl<Msg> From<MouseArea<Msg>> for widget::v1::MouseArea {
    fn from(value: MouseArea<Msg>) -> Self {
        let inter: Option<widget::v1::mouse_area::Interaction> = value.interaction.map(From::from);

        Self {
            child: Some(Box::new(value.child.into())),
            on_press: value.on_press.is_some(),
            on_release: value.on_release.is_some(),
            on_double_click: value.on_double_click.is_some(),
            on_right_press: value.on_right_press.is_some(),
            on_right_release: value.on_right_release.is_some(),
            on_middle_press: value.on_middle_press.is_some(),
            on_middle_release: value.on_middle_release.is_some(),
            on_enter: value.on_enter.is_some(),
            on_scroll: false,
            on_exit: value.on_exit.is_some(),
            on_move: false,
            interaction: inter.map(From::from),
            widget_id: value.widget_id.map(WidgetId::to_inner),
            unique_id: value.unique_id,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Interaction {
    None,
    Idle,
    Pointer,
    Grab,
    Text,
    Crosshair,
    Grabbing,
    ResizeHorizontal,
    ResizeVertical,
    ResizeDiagonalUp,
    ResizeDiagonalDown,
    NotAllowed,
    ZoomIn,
    ZoomOut,
    Cell,
    Move,
    Copy,
    Help,
}

impl From<Interaction> for widget::v1::mouse_area::Interaction {
    fn from(value: Interaction) -> Self {
        match value {
            Interaction::None => Self::None,
            Interaction::Idle => Self::Idle,
            Interaction::Pointer => Self::Pointer,
            Interaction::Grab => Self::Grab,
            Interaction::Text => Self::Text,
            Interaction::Crosshair => Self::Crosshair,
            Interaction::Grabbing => Self::Grabbing,
            Interaction::ResizeHorizontal => Self::ResizeHorizontal,
            Interaction::ResizeVertical => Self::ResizeVertical,
            Interaction::ResizeDiagonalUp => Self::ResizeDiagonalUp,
            Interaction::ResizeDiagonalDown => Self::ResizeDiagonalDown,
            Interaction::NotAllowed => Self::NotAllowed,
            Interaction::ZoomIn => Self::ZoomIn,
            Interaction::ZoomOut => Self::ZoomOut,
            Interaction::Cell => Self::Cell,
            Interaction::Move => Self::Move,
            Interaction::Copy => Self::Copy,
            Interaction::Help => Self::Help,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ScrollDelta {
    Lines {
        x: f32,
        y: f32,
    },
    Pixels {
        x: f32,
        y: f32,
    },
}

//impl From<ScrollDelta> for widget::v1::mouse_area::ScrollDelta {
    //fn from(value: ScrollDelta) -> Self {
        //use widget::v1::mouse_area::scroll_delta;

        //let data = match value {
            //ScrollDelta::Lines {x, y} => scroll_delta::Data::Lines(scroll_delta::Lines {x, y}),
            //ScrollDelta::Pixels { x, y } => scroll_delta::Data::Pixels(scroll_delta::Pixels {x, y}),
        //};

        //Self {
            //data: Some(data)
        //}

    //}
//}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Point {
    x: f32,
    y: f32,
}
