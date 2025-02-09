// SPDX-License-Identifier: GPL-3.0-or-later

pub mod pointer;
pub mod render_elements;
pub mod texture;
pub mod util;

use std::ops::Deref;

use smithay::{
    backend::renderer::{
        element::{surface::WaylandSurfaceRenderElement, AsRenderElements, RenderElementStates},
        gles::GlesRenderer,
        ImportAll, ImportMem, Renderer, Texture,
    },
    desktop::{
        layer_map_for_output,
        space::SpaceElement,
        utils::{
            surface_presentation_feedback_flags_from_states, surface_primary_scanout_output,
            OutputPresentationFeedback,
        },
        PopupManager, Space, WindowSurface,
    },
    output::Output,
    utils::{Logical, Point, Scale},
    wayland::shell::wlr_layer,
};
use util::surface::WlSurfaceTextureRenderElement;

use crate::{
    backend::{udev::UdevRenderer, Backend},
    layout::transaction::SnapshotRenderElement,
    pinnacle_render_elements,
    state::{State, WithState},
    window::WindowElement,
};

use self::{
    pointer::PointerRenderElement, util::surface::texture_render_elements_from_surface_tree,
};

pub const CLEAR_COLOR: [f32; 4] = [0.6, 0.6, 0.6, 1.0];
pub const CLEAR_COLOR_LOCKED: [f32; 4] = [0.2, 0.0, 0.3, 1.0];

pinnacle_render_elements! {
    #[derive(Debug)]
    pub enum OutputRenderElement<R> {
        Surface = WaylandSurfaceRenderElement<R>,
        Pointer = PointerRenderElement<R>,
        Snapshot = SnapshotRenderElement<R>,
    }
}

/// Trait to reduce bound specifications.
pub trait PRenderer
where
    Self: Renderer<TextureId = Self::PTextureId, Error = Self::PError> + ImportAll + ImportMem,
    <Self as Renderer>::TextureId: Texture + Clone + 'static,
{
    // Self::TextureId: Texture + Clone + 'static doesn't work in the where clause,
    // which is why these associated types exist.
    //
    // From https://github.com/YaLTeR/niri/blob/ae7fb4c4f405aa0ff49930040d414581a812d938/src/render_helpers/renderer.rs#L10
    type PTextureId: Texture + Clone + Send + 'static;
    type PError: std::error::Error + Send + Sync + 'static;
}

impl<R> PRenderer for R
where
    R: ImportAll + ImportMem,
    R::TextureId: Texture + Clone + Send + 'static,
    R::Error: std::error::Error + Send + Sync + 'static,
{
    type PTextureId = R::TextureId;
    type PError = R::Error;
}

/// Trait for renderers that provide [`GlesRenderer`]s.
pub trait AsGlesRenderer {
    /// Gets a [`GlesRenderer`] from this renderer.
    fn as_gles_renderer(&mut self) -> &mut GlesRenderer;
}

impl AsGlesRenderer for GlesRenderer {
    fn as_gles_renderer(&mut self) -> &mut GlesRenderer {
        self
    }
}

impl AsGlesRenderer for UdevRenderer<'_> {
    fn as_gles_renderer(&mut self) -> &mut GlesRenderer {
        self.as_mut()
    }
}

impl WindowElement {
    /// Render elements for this window at the given *logical* location in the space,
    /// output-relative.
    pub fn render_elements<R: PRenderer>(
        &self,
        renderer: &mut R,
        location: Point<i32, Logical>, // TODO: make f64
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<WaylandSurfaceRenderElement<R>> {
        let _span = tracy_client::span!("WindowElement::render_elements");

        let location = location - self.geometry().loc;
        let phys_loc = location.to_f64().to_physical_precise_round(scale);
        self.deref()
            .render_elements(renderer, phys_loc, scale, alpha)
    }

    /// Render elements for this window as textures.
    pub fn texture_render_elements<R: PRenderer + AsGlesRenderer>(
        &self,
        renderer: &mut R,
        location: Point<i32, Logical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<WlSurfaceTextureRenderElement> {
        let _span = tracy_client::span!("WindowElement::texture_render_elements");

        let location = location - self.geometry().loc;
        let location = location.to_f64().to_physical_precise_round(scale);

        match self.underlying_surface() {
            WindowSurface::Wayland(s) => {
                let mut render_elements = Vec::new();
                let surface = s.wl_surface();
                let popup_render_elements =
                    PopupManager::popups_for_surface(surface).flat_map(|(popup, popup_offset)| {
                        let offset = (self.geometry().loc + popup_offset - popup.geometry().loc)
                            .to_physical_precise_round(scale);

                        texture_render_elements_from_surface_tree(
                            renderer.as_gles_renderer(),
                            popup.wl_surface(),
                            location + offset,
                            scale,
                            alpha,
                        )
                    });

                render_elements.extend(popup_render_elements);

                render_elements.extend(texture_render_elements_from_surface_tree(
                    renderer.as_gles_renderer(),
                    surface,
                    location,
                    scale,
                    alpha,
                ));

                render_elements
            }
            WindowSurface::X11(s) => {
                if let Some(surface) = s.wl_surface() {
                    texture_render_elements_from_surface_tree(
                        renderer.as_gles_renderer(),
                        &surface,
                        location,
                        scale,
                        alpha,
                    )
                } else {
                    Vec::new()
                }
            }
        }
    }
}

struct LayerRenderElements<R: PRenderer> {
    background: Vec<WaylandSurfaceRenderElement<R>>,
    bottom: Vec<WaylandSurfaceRenderElement<R>>,
    top: Vec<WaylandSurfaceRenderElement<R>>,
    overlay: Vec<WaylandSurfaceRenderElement<R>>,
}

fn layer_render_elements<R: PRenderer>(
    output: &Output,
    renderer: &mut R,
    scale: Scale<f64>,
) -> LayerRenderElements<R> {
    let _span = tracy_client::span!("layer_render_elements");

    let layer_map = layer_map_for_output(output);
    let mut overlay = vec![];
    let mut top = vec![];
    let mut bottom = vec![];
    let mut background = vec![];

    let layer_elements = layer_map
        .layers()
        .rev()
        .filter_map(|surface| {
            layer_map
                .layer_geometry(surface)
                .map(|geo| (surface, geo.loc))
        })
        .map(|(surface, loc)| {
            let loc = loc.to_physical_precise_round(scale);
            let render_elements = surface
                .render_elements::<WaylandSurfaceRenderElement<R>>(renderer, loc, scale, 1.0);
            (surface.layer(), render_elements)
        });

    for (layer, elements) in layer_elements {
        match layer {
            wlr_layer::Layer::Background => background.extend(elements),
            wlr_layer::Layer::Bottom => bottom.extend(elements),
            wlr_layer::Layer::Top => top.extend(elements),
            wlr_layer::Layer::Overlay => overlay.extend(elements),
        }
    }

    LayerRenderElements {
        background,
        bottom,
        top,
        overlay,
    }
}

/// Get render elements for windows on active tags.
///
/// ret.1 contains render elements for the windows at and above the first fullscreen window.
/// ret.2 contains the rest.
fn window_render_elements<'a, I, R: PRenderer>(
    output: &Output,
    windows: I,
    space: &Space<WindowElement>,
    renderer: &mut R,
    scale: Scale<f64>,
) -> (Vec<OutputRenderElement<R>>, Vec<OutputRenderElement<R>>)
where
    I: IntoIterator<Item = &'a WindowElement>,
    I::IntoIter: DoubleEndedIterator,
{
    let _span = tracy_client::span!("window_render_elements");

    let mut last_fullscreen_split_at = 0;

    let mut fullscreen_and_up = windows
        .into_iter()
        .rev() // rev because I treat the focus stack backwards vs how the renderer orders it
        .enumerate()
        .map(|(i, win)| {
            win.with_state_mut(|state| state.offscreen_elem_id.take());

            if win.with_state(|state| state.window_state.is_fullscreen()) {
                last_fullscreen_split_at = i + 1;
            }

            let loc = space.element_location(win).unwrap_or_default() - output.current_location();

            win.render_elements(renderer, loc, scale, 1.0)
                .into_iter()
                .map(OutputRenderElement::from)
        }).collect::<Vec<_>>();

    let rest = fullscreen_and_up.split_off(last_fullscreen_split_at);

    (
        fullscreen_and_up.into_iter().flatten().collect(),
        rest.into_iter().flatten().collect(),
    )
}

/// Generate render elements for the given output.
///
/// Render elements will be pulled from the provided windows,
/// with the first window being at the top and subsequent ones beneath.
pub fn output_render_elements<R: PRenderer + AsGlesRenderer>(
    output: &Output,
    renderer: &mut R,
    space: &Space<WindowElement>,
    windows: &[WindowElement],
) -> Vec<OutputRenderElement<R>> {
    let _span = tracy_client::span!("output_render_elements");

    let scale = Scale::from(output.current_scale().fractional_scale());

    let mut output_render_elements: Vec<OutputRenderElement<_>> = Vec::new();

    // // draw input method surface if any
    // let rectangle = input_method.coordinates();
    // let position = Point::from((
    //     rectangle.loc.x + rectangle.size.w,
    //     rectangle.loc.y + rectangle.size.h,
    // ));
    // input_method.with_surface(|surface| {
    //     custom_render_elements.extend(AsRenderElements::<R>::render_elements(
    //         &SurfaceTree::from_surface(surface),
    //         renderer,
    //         position.to_physical_precise_round(scale),
    //         scale,
    //         1.0,
    //     ));
    // });

    let output_loc = output.current_location();

    let LayerRenderElements {
        background,
        bottom,
        top,
        overlay,
    } = layer_render_elements(output, renderer, scale);

    let fullscreen_and_up_elements;
    let rest_of_window_elements;

    // If there is a snapshot, render its elements instead
    if let Some((fs_and_up_elements, under_fs_elements)) = output.with_state(|state| {
        state
            .layout_transaction
            .as_ref()
            .map(|ts| ts.render_elements(renderer, space, output_loc, scale, 1.0))
    }) {
        fullscreen_and_up_elements = fs_and_up_elements
            .into_iter()
            .map(OutputRenderElement::from)
            .collect();
        rest_of_window_elements = under_fs_elements
            .into_iter()
            .map(OutputRenderElement::from)
            .collect();
    } else {
        let windows = windows.iter().filter(|win| win.is_on_active_tag());
        (fullscreen_and_up_elements, rest_of_window_elements) =
            window_render_elements::<_, R>(output, windows, space, renderer, scale);
    }

    // Elements render from top to bottom

    output_render_elements.extend(overlay.into_iter().map(OutputRenderElement::from));
    output_render_elements.extend(fullscreen_and_up_elements);
    output_render_elements.extend(top.into_iter().map(OutputRenderElement::from));
    output_render_elements.extend(rest_of_window_elements);
    output_render_elements.extend(bottom.into_iter().map(OutputRenderElement::from));
    output_render_elements.extend(background.into_iter().map(OutputRenderElement::from));

    output_render_elements
}

// TODO: docs
pub fn take_presentation_feedback(
    output: &Output,
    space: &Space<WindowElement>,
    render_element_states: &RenderElementStates,
) -> OutputPresentationFeedback {
    let _span = tracy_client::span!("take_presentation_feedback");

    let mut output_presentation_feedback = OutputPresentationFeedback::new(output);

    space.elements().for_each(|window| {
        if space.outputs_for_element(window).contains(output) {
            window.take_presentation_feedback(
                &mut output_presentation_feedback,
                surface_primary_scanout_output,
                |surface, _| {
                    surface_presentation_feedback_flags_from_states(surface, render_element_states)
                },
            );
        }
    });

    let map = smithay::desktop::layer_map_for_output(output);
    for layer_surface in map.layers() {
        layer_surface.take_presentation_feedback(
            &mut output_presentation_feedback,
            surface_primary_scanout_output,
            |surface, _| {
                surface_presentation_feedback_flags_from_states(surface, render_element_states)
            },
        );
    }

    output_presentation_feedback
}

impl State {
    /// Schedule a new render.
    pub fn schedule_render(&mut self, output: &Output) {
        let _span = tracy_client::span!("State::schedule_render");

        match &mut self.backend {
            Backend::Udev(udev) => {
                udev.schedule_render(output);
            }
            Backend::Winit(winit) => {
                winit.schedule_render();
            }
            #[cfg(feature = "testing")]
            Backend::Dummy(_) => (),
        }
    }
}
