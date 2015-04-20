
use graphics::Graphics;
use graphics::character::{Character, CharacterCache};
use label::FontSize;
use mouse::{ButtonState, Mouse};
use piston::input;
use piston::event::{
    GenericEvent,
    MouseCursorEvent,
    PressEvent,
    ReleaseEvent,
    RenderEvent,
    TextEvent,
};
use position::{Dimensions, HorizontalAlign, Point, Position, VerticalAlign};
use std::iter::repeat;
use theme::Theme;
use widget::Kind as WidgetKind;
use widget::Widget;

/// User interface identifier. Each widget must use a unique `UiId` so that it's state can be
/// cached within the `Ui` type. The reason we use a usize is because widgets are cached within
/// a `Vec`, which is limited to a size of `usize` elements.
pub type UiId = usize;

/// `Ui` is the most important type within Conrod and is necessary for rendering and maintaining
/// widget state.
/// # Ui Handles the following:
/// * Contains the state of all widgets which can be indexed via their UiId.
/// * Stores rendering state for each widget until the end of each render cycle.
/// * Contains the theme used for default styling of the widgets.
/// * Maintains the latest user input state (for mouse and keyboard).
/// * Maintains the latest window dimensions.
pub struct Ui<C> {
    /// The Widget cache, storing state for all widgets.
    widget_cache: Vec<Widget>,
    /// The theme used to set default styling for widgets.
    pub theme: Theme,
    /// The latest received mouse state.
    pub mouse: Mouse,
    /// Keys that have been pressed since the end of the last render cycle.
    pub keys_just_pressed: Vec<input::keyboard::Key>,
    /// Keys that have been released since the end of the last render cycle.
    pub keys_just_released: Vec<input::keyboard::Key>,
    /// Text that has been entered since the end of the last render cycle.
    pub text_just_entered: Vec<String>,
    character_cache: C,
    prev_event_was_render: bool,
    /// Window width.
    pub win_w: f64,
    /// Window height.
    pub win_h: f64,
    /// The UiId of the previously drawn Widget.
    maybe_prev_ui_id: Option<UiId>,
}

impl<C> Ui<C> {

    /// Constructor for a UiContext.
    pub fn new(character_cache: C, theme: Theme) -> Ui<C> {
        Ui {
            widget_cache: repeat(Widget::empty()).take(512).collect(),
            theme: theme,
            mouse: Mouse::new([0.0, 0.0], ButtonState::Up, ButtonState::Up, ButtonState::Up),
            keys_just_pressed: Vec::with_capacity(10),
            keys_just_released: Vec::with_capacity(10),
            text_just_entered: Vec::with_capacity(10),
            character_cache: character_cache,
            prev_event_was_render: false,
            win_w: 0.0,
            win_h: 0.0,
            maybe_prev_ui_id: None,
        }
    }

    /// Handle game events and update the state.
    pub fn handle_event<E: GenericEvent + ::std::fmt::Debug>(&mut self, event: &E) {
        if self.prev_event_was_render {
            self.flush_input();
            self.maybe_prev_ui_id = None;
            self.prev_event_was_render = false;
        }

        event.render(|args| {
            self.win_w = args.width as f64;
            self.win_h = args.height as f64;
            self.prev_event_was_render = true;
        });

        event.mouse_cursor(|x, y| {
            // Convert mouse coords to (0, 0) origin.
            self.mouse.xy = [x - self.win_w / 2.0, -(y - self.win_h / 2.0)];
        });

        event.press(|button_type| {
            use piston::input::Button;
            use piston::input::MouseButton::Left;

            match button_type {
                Button::Mouse(button) => {
                    *match button {
                        Left => &mut self.mouse.left,
                        _/*input::mouse::Right*/ => &mut self.mouse.right,
                        //Middle => &mut self.mouse.middle,
                    } = ButtonState::Down;
                },
                Button::Keyboard(key) => self.keys_just_pressed.push(key),
            }
        });

        event.release(|button_type| {
            use piston::input::Button;
            use piston::input::MouseButton::Left;
            match button_type {
                Button::Mouse(button) => {
                    *match button {
                        Left => &mut self.mouse.left,
                        _/*input::mouse::Right*/ => &mut self.mouse.right,
                        //Middle => &mut self.mouse.middle,
                    } = ButtonState::Up;
                },
                Button::Keyboard(key) => self.keys_just_released.push(key),
            }
        });

        event.text(|text| {
            self.text_just_entered.push(text.to_string())
        });
    }

    /// Return a reference to a `Character` from the GlyphCache.
    pub fn get_character(&mut self,
                         size: FontSize,
                         ch: char) -> &Character<C::Texture>
        where
            C: CharacterCache
    {
        self.character_cache.character(size, ch)
    }

    /// Return the width of a 'Character'.
    pub fn get_character_w(&mut self, size: FontSize, ch: char) -> f64
        where
            C: CharacterCache
    {
        self.get_character(size, ch).width()
    }

    /// Flush all stored keys.
    pub fn flush_input(&mut self) {
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
        self.text_just_entered.clear();
    }

    /// Return the current mouse state.
    pub fn get_mouse_state(&self) -> Mouse {
        self.mouse
    }

    /// Return the vector of recently pressed keys.
    pub fn get_pressed_keys(&self) -> Vec<input::keyboard::Key> {
        self.keys_just_pressed.clone()
    }

    /// Return the vector of recently entered text.
    pub fn get_entered_text(&self) -> Vec<String> {
        self.text_just_entered.clone()
    }

    /// Return a mutable reference to the widget that matches the given ui_id
    pub fn get_widget(&mut self, ui_id: UiId, default: WidgetKind) -> &mut WidgetKind {
        let ui_id_idx = ui_id as usize;
        if self.widget_cache.len() > ui_id_idx {
            match &mut self.widget_cache[ui_id_idx].kind {
                &mut WidgetKind::NoWidget => {
                    let mut widget = &mut self.widget_cache[ui_id_idx].kind;
                    *widget = default;
                    widget
                },
                _ => &mut self.widget_cache[ui_id_idx].kind,
            }
        } else {
            if ui_id_idx >= self.widget_cache.len() {
                let num_to_extend = ui_id_idx - self.widget_cache.len();
                self.widget_cache.extend(repeat(Widget::empty())
                    .take(num_to_extend)
                    .chain(Some(Widget::new(default)).into_iter()));
            } else {
                self.widget_cache[ui_id_idx] = Widget::new(default);
            }
            &mut self.widget_cache[ui_id_idx].kind
        }
    }

    /// Set the given widget at the given UiId.
    pub fn set_widget(&mut self, ui_id: UiId, widget: Widget) {
        if self.widget_cache[ui_id].kind.matches(&widget.kind)
        || self.widget_cache[ui_id].kind.matches(&WidgetKind::NoWidget) {
            self.widget_cache[ui_id] = widget;
            self.maybe_prev_ui_id = Some(ui_id);
        } else {
            panic!("A widget of a different kind already exists at the given UiId ({:?}).
                    You tried to insert a {:?}, however the existing widget is a {:?}.
                    Check your widgets' `UiId`s for errors.",
                    ui_id, &widget.kind, &self.widget_cache[ui_id].kind);
        }
    }

    /// Get the centred xy coords for some given `Dimension`s, `Position` and alignment.
    pub fn get_xy(&self,
                  position: Position,
                  dim: Dimensions,
                  h_align: HorizontalAlign,
                  v_align: VerticalAlign) -> Point {
        match position {
            Position::Absolute(x, y) => [x, y],
            Position::Relative(x, y, maybe_ui_id) => {
                match maybe_ui_id.or(self.maybe_prev_ui_id) {
                    None => [0.0, 0.0],
                    Some(rel_ui_id) => ::vecmath::vec2_add(self.widget_cache[rel_ui_id].xy, [x, y]),
                }
            },
            Position::Direction(direction, maybe_ui_id) => {
                use position::{align_left_of, align_right_of, align_top_of, align_bottom_of};
                match maybe_ui_id.or(self.maybe_prev_ui_id) {
                    None => [0.0, 0.0],
                    Some(rel_ui_id) => {
                        use position::Direction;
                        let rel_xy = self.widget_cache[rel_ui_id].xy;
                        if let Some(ref element) = self.widget_cache[rel_ui_id].element {
                            let (rel_w, rel_h) = element.get_size();
                            let (rel_w, rel_h) = (rel_w as f64, rel_h as f64);
                            match direction {

                                Direction::Up(px) => {
                                    let x = rel_xy[0] + match h_align {
                                        HorizontalAlign::Middle => 0.0,
                                        HorizontalAlign::Left   => align_left_of(rel_w, dim[0]),
                                        HorizontalAlign::Right  => align_right_of(rel_w, dim[0]),
                                    };
                                    let y = rel_xy[1] + rel_h / 2.0 + dim[1] / 2.0 + px;
                                    [x, y]
                                },

                                Direction::Down(px) => {
                                    let x = rel_xy[0] + match h_align {
                                        HorizontalAlign::Middle => 0.0,
                                        HorizontalAlign::Left   => align_left_of(rel_w, dim[0]),
                                        HorizontalAlign::Right  => align_right_of(rel_w, dim[0]),
                                    };
                                    let y = rel_xy[1] - rel_h / 2.0 - dim[1] / 2.0 - px;
                                    [x, y]
                                },

                                Direction::Left(px) => {
                                    let y = rel_xy[1] + match v_align {
                                        VerticalAlign::Middle => 0.0,
                                        VerticalAlign::Bottom => align_bottom_of(rel_h, dim[1]),
                                        VerticalAlign::Top    => align_top_of(rel_h, dim[1]),
                                    };
                                    let x = rel_xy[0] - rel_w / 2.0 - dim[0] / 2.0 - px;
                                    [x, y]
                                },

                                Direction::Right(px) => {
                                    let y = rel_xy[1] + match v_align {
                                        VerticalAlign::Middle => 0.0,
                                        VerticalAlign::Bottom => align_bottom_of(rel_h, dim[1]),
                                        VerticalAlign::Top    => align_top_of(rel_h, dim[1]),
                                    };
                                    let x = rel_xy[0] + rel_w / 2.0 + dim[0] / 2.0 + px;
                                    [x, y]
                                },

                            }
                        } else {
                            rel_xy
                        }
                    },
                }
            },
        }
    }

    /// Draw the `Ui` in it's current state.
    /// - Sort widgets by render depth (depth first).
    /// - Construct the elmesque `Renderer` for rendering the elm `Element`s.
    /// - Render all widgets.
    pub fn draw<G>(&mut self, graphics: &mut G)
        where
            C: CharacterCache,
            G: Graphics<Texture = C::Texture>,
    {
        use elmesque::Renderer;
        use std::cmp::Ordering;
        let Ui { ref mut widget_cache, ref win_w, ref win_h, ref mut character_cache, .. } = *self;
        let mut widgets: Vec<_> = widget_cache.iter_mut().collect();
        widgets.sort_by(|a, b| if      a.depth < b.depth { Ordering::Greater }
                               else if a.depth > b.depth { Ordering::Less }
                               else                      { Ordering::Equal });
        let mut renderer = Renderer::new(*win_w, *win_h, graphics).character_cache(character_cache);
        for widget in widgets.into_iter() {
            if let Some(element) = widget.element.take() {
                element.draw(&mut renderer);
            }
        }
    }

}