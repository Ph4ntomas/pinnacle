// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Tag management.
//!
//! This module allows you to interact with Pinnacle's tag system.
//!
//! # The Tag System
//! Many Wayland compositors use workspaces for window management.
//! Each window is assigned to a workspace and will only show up if that workspace is being
//! viewed. This is a find way to manage windows, but it's not that powerful.
//!
//! Instead, Pinnacle works with a tag system similar to window managers like [dwm](https://dwm.suckless.org/)
//! and, the window manager Pinnacle takes inspiration from, [awesome](https://awesomewm.org/).
//!
//! In a tag system, there are no workspaces. Instead, each window can be tagged with zero or more
//! tags, and zero or more tags can be displayed on a monitor at once. This allows you to, for
//! example, bring in your browsers on the same screen as your IDE by toggling the "Browser" tag.
//!
//! Workspaces can be emulated by only displaying one tag at a time. Combining this feature with
//! the ability to tag windows with multiple tags allows you to have one window show up on multiple
//! different "workspaces". As you can see, this system is much more powerful than workspaces
//! alone.
//!
//! # Configuration
//! `tag` contains the [`Tag`] struct, which allows you to add new tags
//! and get handles to already defined ones.
//!
//! These [`TagHandle`]s allow you to manipulate individual tags and get their properties.

use futures::FutureExt;
use pinnacle_api_defs::pinnacle::{
    tag::v1::{
        AddRequest, GetActiveRequest, GetNameRequest, GetOutputNameRequest, GetRequest,
        RemoveRequest, SetActiveRequest, SwitchToRequest,
    },
    util::v1::SetOrToggle,
};

use crate::{
    client::Client,
    output::OutputHandle,
    signal::{SignalHandle, TagSignal},
    signal_module,
    util::Batch,
    BlockOnTokio,
};

/// Add tags to the specified output.
///
/// This will add tags with the given names to `output` and return [`TagHandle`]s to all of
/// them.
///
/// # Examples
///
/// ```
/// // Add tags 1-5 to the focused output
/// if let Some(op) = output.get_focused() {
///     let tags = tag.add(&op, ["1", "2", "3", "4", "5"]);
/// }
/// ```
pub fn add(
    output: &OutputHandle,
    tag_names: impl IntoIterator<Item = impl ToString>,
) -> impl Iterator<Item = TagHandle> {
    let output_name = output.name();
    let tag_names = tag_names.into_iter().map(|name| name.to_string()).collect();

    Client::tag()
        .add(AddRequest {
            output_name,
            tag_names,
        })
        .block_on_tokio()
        .unwrap()
        .into_inner()
        .tag_ids
        .into_iter()
        .map(|id| TagHandle { id })
}

pub fn get_all() -> impl Iterator<Item = TagHandle> {
    get_all_async().block_on_tokio()
}

/// Get handles to all tags across all outputs.
///
/// # Examples
///
/// ```
///
///
/// let all_tags = tag.get_all();
/// ```
pub async fn get_all_async() -> impl Iterator<Item = TagHandle> {
    Client::tag()
        .get(GetRequest {})
        .await
        .unwrap()
        .into_inner()
        .tag_ids
        .into_iter()
        .map(|id| TagHandle { id })
}

pub fn get(name: impl ToString) -> Option<TagHandle> {
    get_async(name).block_on_tokio()
}

/// Get a handle to the first tag with the given name on the focused output.
///
/// If you need to get a tag on a specific output, see [`Tag::get_on_output`].
///
/// # Examples
///
/// ```
/// // Get tag "Thing" on the focused output
///
///
/// let tg = tag.get("Thing");
/// ```
pub async fn get_async(name: impl ToString) -> Option<TagHandle> {
    let name = name.to_string();
    let focused_op = crate::output::get_focused_async().await?;

    get_on_output_async(name, &focused_op).await
}

pub fn get_on_output(name: impl ToString, output: &OutputHandle) -> Option<TagHandle> {
    get_on_output_async(name, output).block_on_tokio()
}

/// Get a handle to the first tag with the given name on the specified output.
///
/// If you just need to get a tag on the focused output, see [`Tag::get`].
///
/// # Examples
///
/// ```
/// // Get tag "Thing" on "HDMI-1"
///
///
/// let tg = tag.get_on_output("Thing", output.get_by_name("HDMI-2")?);
/// ```
pub async fn get_on_output_async(name: impl ToString, output: &OutputHandle) -> Option<TagHandle> {
    let name = name.to_string();
    let output = output.clone();
    get_all_async().await.batch_find(
        |tag| async { (tag.name_async().await, tag.output_async().await) }.boxed(),
        |(n, op)| *n == name && *op == output,
    )
}

/// Remove the given tags from their outputs.
///
/// # Examples
///
/// ```
/// let tags = tag.add(output.get_by_name("DP-1")?, ["1", "2", "Buckle", "Shoe"]);
///
/// tag.remove(tags); // "DP-1" no longer has any tags
/// ```
pub fn remove(tags: impl IntoIterator<Item = TagHandle>) {
    let tag_ids = tags.into_iter().map(|handle| handle.id).collect::<Vec<_>>();

    Client::tag()
        .remove(RemoveRequest { tag_ids })
        .block_on_tokio()
        .unwrap();
}

/// Connect to a tag signal.
///
/// The compositor will fire off signals that your config can listen for and act upon.
/// You can pass in a [`TagSignal`] along with a callback and it will get run
/// with the necessary arguments every time a signal of that type is received.
pub fn connect_signal(signal: TagSignal) -> SignalHandle {
    let mut signal_state = signal_module();

    match signal {
        TagSignal::Active(f) => signal_state.tag_active.add_callback(f),
    }
}

/// A handle to a tag.
///
/// This handle allows you to do things like switch to tags and get their properties.
#[derive(Debug, Clone, Copy)]
pub struct TagHandle {
    pub(crate) id: u32,
}

impl PartialEq for TagHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TagHandle {}

impl std::hash::Hash for TagHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl TagHandle {
    /// Activate this tag and deactivate all other ones on the same output.
    ///
    /// This essentially emulates what a traditional workspace is.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assume the focused output has the following inactive tags and windows:
    /// // "1": Alacritty
    /// // "2": Firefox, Discord
    /// // "3": Steam
    /// tag.get("2")?.switch_to(); // Displays Firefox and Discord
    /// tag.get("3")?.switch_to(); // Displays Steam
    /// ```
    pub fn switch_to(&self) {
        let tag_id = self.id;

        Client::tag()
            .switch_to(SwitchToRequest { tag_id })
            .block_on_tokio()
            .unwrap();
    }

    /// Set this tag to active or not.
    ///
    /// While active, windows with this tag will be displayed.
    ///
    /// While inactive, windows with this tag will not be displayed unless they have other active
    /// tags.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assume the focused output has the following inactive tags and windows:
    /// // "1": Alacritty
    /// // "2": Firefox, Discord
    /// // "3": Steam
    /// tag.get("2")?.set_active(true);  // Displays Firefox and Discord
    /// tag.get("3")?.set_active(true);  // Displays Firefox, Discord, and Steam
    /// tag.get("2")?.set_active(false); // Displays Steam
    /// ```
    pub fn set_active(&self, set: bool) {
        let tag_id = self.id;

        Client::tag()
            .set_active(SetActiveRequest {
                tag_id,
                set_or_toggle: match set {
                    true => SetOrToggle::Set,
                    false => SetOrToggle::Unset,
                }
                .into(),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Toggle this tag between active and inactive.
    ///
    /// While active, windows with this tag will be displayed.
    ///
    /// While inactive, windows with this tag will not be displayed unless they have other active
    /// tags.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assume the focused output has the following inactive tags and windows:
    /// // "1": Alacritty
    /// // "2": Firefox, Discord
    /// // "3": Steam
    /// tag.get("2")?.toggle(); // Displays Firefox and Discord
    /// tag.get("3")?.toggle(); // Displays Firefox, Discord, and Steam
    /// tag.get("3")?.toggle(); // Displays Firefox, Discord
    /// tag.get("2")?.toggle(); // Displays nothing
    /// ```
    pub fn toggle_active(&self) {
        let tag_id = self.id;

        Client::tag()
            .set_active(SetActiveRequest {
                tag_id,
                set_or_toggle: SetOrToggle::Toggle.into(),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Remove this tag from its output.
    ///
    /// # Examples
    ///
    /// ```
    /// let tags = tag
    ///     .add(output.get_by_name("DP-1")?, ["1", "2", "Buckle", "Shoe"])
    ///     .collect::<Vec<_>>;
    ///
    /// tags[1].remove();
    /// tags[3].remove();
    /// // "DP-1" now only has tags "1" and "Buckle"
    /// ```
    pub fn remove(&self) {
        let tag_id = self.id;

        Client::tag()
            .remove(RemoveRequest {
                tag_ids: vec![tag_id],
            })
            .block_on_tokio()
            .unwrap();
    }

    pub fn active(&self) -> bool {
        self.active_async().block_on_tokio()
    }

    /// Get this tag's active status.
    ///
    ///
    ///
    /// Shorthand for `self.props().active`.
    pub async fn active_async(&self) -> bool {
        let tag_id = self.id;

        Client::tag()
            .get_active(GetActiveRequest { tag_id })
            .await
            .unwrap()
            .into_inner()
            .active
    }

    pub fn name(&self) -> String {
        self.name_async().block_on_tokio()
    }

    /// Get this tag's name.
    ///
    ///
    ///
    /// Shorthand for `self.props().name`.
    pub async fn name_async(&self) -> String {
        let tag_id = self.id;

        Client::tag()
            .get_name(GetNameRequest { tag_id })
            .await
            .unwrap()
            .into_inner()
            .name
    }

    pub fn output(&self) -> OutputHandle {
        self.output_async().block_on_tokio()
    }

    /// Get a handle to the output this tag is on.
    ///
    ///
    ///
    /// Shorthand for `self.props().output`.
    pub async fn output_async(&self) -> OutputHandle {
        let tag_id = self.id;

        let name = Client::tag()
            .get_output_name(GetOutputNameRequest { tag_id })
            .await
            .unwrap()
            .into_inner()
            .output_name;
        OutputHandle { name }
    }

    // TODO:
    /// Get all windows with this tag.
    ///
    /// Shorthand for `self.props().windows`.
    // pub fn windows(&self) -> Vec<WindowHandle> {
    //     self.props().windows
    // }
    //
    // /// The async version of [`TagHandle::windows`].
    // pub async fn windows_async(&self) -> Vec<WindowHandle> {
    //     self.props_async().await.windows
    // }

    /// Get this tag's raw compositor id.
    pub fn id(&self) -> u32 {
        self.id
    }
}
