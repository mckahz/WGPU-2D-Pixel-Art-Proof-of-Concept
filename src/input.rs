use gilrs::*;
use winit::event::{KeyboardInput, VirtualKeyCode};

#[derive(Debug)]
pub struct Key<'a> {
    pub mappings: (&'a [VirtualKeyCode], &'a [gilrs::Button]),
    pub just_changed: bool,
    pub pressed: bool,
}

impl<'a> Key<'a> {
    pub fn just_pressed(&self) -> bool {
        self.just_changed && self.pressed
    }
    #[allow(dead_code)]
    pub fn just_released(&self) -> bool {
        self.just_changed && !self.pressed
    }
}

#[derive(Debug)]
pub struct Input<'a> {
    pub jump: Key<'a>,
    pub left: Key<'a>,
    pub right: Key<'a>,
    pub attack: Key<'a>,
}

impl<'a> Input<'a> {
    pub fn update(
        &mut self,
        controller_inputs: &mut Vec<gilrs::Event>,
        keyboard_inputs: &mut Vec<winit::event::KeyboardInput>,
    ) {
        //let inputs = vec![];
        let controller_info: Vec<(Button, bool)> = controller_inputs
            .into_iter()
            .map(|e| match e.event {
                EventType::ButtonPressed(button, _) => Some((button, true)),
                EventType::ButtonReleased(button, _) => Some((button, false)),
                _ => None,
            })
            .filter(|x| x.is_some())
            .map(|x| x.unwrap())
            .collect();
        let keyboard_info: Vec<(VirtualKeyCode, bool)> = keyboard_inputs
            .into_iter()
            .map(|e| {
                (
                    e.virtual_keycode,
                    match e.state {
                        winit::event::ElementState::Pressed => true,
                        winit::event::ElementState::Released => false,
                    },
                )
            })
            .filter(|(b, _)| b.is_some())
            .map(|(b, p)| (b.unwrap(), p))
            .collect();
        for action in [
            &mut self.jump,
            &mut self.left,
            &mut self.right,
            &mut self.attack,
        ] {
            action.just_changed = false;
            let (keys, buttons) = action.mappings;
            for key in keys {
                for (k, p) in keyboard_info.iter() {
                    if *k == *key {
                        action.just_changed = *p != action.pressed;
                        action.pressed = *p;
                    }
                }
            }
            for button in buttons {
                for (b, p) in controller_info.iter() {
                    if *b == *button {
                        action.just_changed = *p != action.pressed;
                        action.pressed = *p;
                    }
                }
            }
        }
    }
}

impl<'a> Default for Input<'a> {
    fn default() -> Self {
        Self {
            jump: Key {
                mappings: (&[VirtualKeyCode::Space], &[Button::South]),
                just_changed: false,
                pressed: false,
            },
            left: Key {
                mappings: (&[VirtualKeyCode::A], &[Button::DPadLeft]),
                just_changed: false,
                pressed: false,
            },
            right: Key {
                mappings: (&[VirtualKeyCode::D], &[Button::DPadRight]),
                just_changed: false,
                pressed: false,
            },
            attack: Key {
                mappings: (&[VirtualKeyCode::W], &[Button::West]),
                just_changed: false,
                pressed: false,
            },
        }
    }
}
