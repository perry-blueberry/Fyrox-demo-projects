//! Game project.
use fyrox::{
    animation::spritesheet::SpriteSheetAnimation,
    core::{
        algebra::{Vector2, Vector3},
        log::Log,
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
        TypeUuidProvider,
    },
    engine::GraphicsContext,
    event::{ElementState, Event, WindowEvent},
    event_loop::ControlFlow,
    gui::{
        message::MessageDirection,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        UiNode,
    },
    impl_component_provider,
    keyboard::KeyCode,
    plugin::{Plugin, PluginConstructor, PluginContext, PluginRegistrationContext},
    scene::{
        dim2::{rectangle::Rectangle, rigidbody::RigidBody},
        loader::AsyncSceneLoader,
        node::Node,
        Scene,
    },
    script::{ScriptContext, ScriptTrait},
};

pub struct GameConstructor;

impl TypeUuidProvider for GameConstructor {
    fn type_uuid() -> Uuid {
        // Ideally this should be unique per-project.
        uuid!("cb358b1c-fc23-4c44-9e59-0a9671324196")
    }
}

impl PluginConstructor for GameConstructor {
    fn register(&self, context: PluginRegistrationContext) {
        let script_constructors = &context.serialization_context.script_constructors;
        script_constructors.add::<Player>("Player");
    }

    fn create_instance(
        &self,
        override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        Box::new(Game::new(override_scene, context))
    }
}

pub struct Game {
    scene: Handle<Scene>,
    loader: Option<AsyncSceneLoader>,
    debug_text: Handle<UiNode>,
}

impl Game {
    pub fn new(override_scene: Handle<Scene>, context: PluginContext) -> Self {
        let mut loader = None;
        let scene = if override_scene.is_some() {
            override_scene
        } else {
            loader = Some(AsyncSceneLoader::begin_loading(
                "data/scene.rgs".into(),
                context.serialization_context.clone(),
                context.resource_manager.clone(),
            ));
            Default::default()
        };

        let debug_text =
            TextBuilder::new(WidgetBuilder::new()).build(&mut context.user_interface.build_ctx());

        Self {
            scene,
            loader,
            debug_text,
        }
    }
}

impl Plugin for Game {
    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        if let Some(loader) = self.loader.as_ref() {
            if let Some(result) = loader.fetch_result() {
                match result {
                    Ok(scene) => {
                        self.scene = context.scenes.add(scene);
                    }
                    Err(err) => Log::err(err),
                }
            }
        }

        if let GraphicsContext::Initialized(graphics_context) = context.graphics_context {
            context.user_interface.send_message(TextMessage::text(
                self.debug_text,
                MessageDirection::ToWidget,
                format!("{}", graphics_context.renderer.get_statistics()),
            ));
        }
    }
}

#[derive(Visit, Reflect, Debug, Clone)]
struct Player {
    sprite: Handle<Node>,
    move_left: bool,
    move_right: bool,
    jump: bool,
    animations: Vec<SpriteSheetAnimation>,
    current_animation: u32,
}

impl_component_provider!(Player,);

impl Default for Player {
    fn default() -> Self {
        Self {
            sprite: Handle::NONE,
            move_left: false,
            move_right: false,
            jump: false,
            animations: Default::default(),
            current_animation: 0,
        }
    }
}

impl TypeUuidProvider for Player {
    // Returns unique script id for serialization needs.
    fn type_uuid() -> Uuid {
        uuid!("c5671d19-9f1a-4286-8486-add4ebaadaec")
    }
}

impl ScriptTrait for Player {
    // Called everytime when there is an event from OS (mouse click, key press, etc.)
    fn on_os_event(&mut self, event: &Event<()>, _context: &mut ScriptContext) {
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::KeyboardInput { event: input, .. } = event {
                let is_pressed = input.state == ElementState::Pressed;

                match input.physical_key {
                    KeyCode::KeyA => self.move_left = is_pressed,
                    KeyCode::KeyD => self.move_right = is_pressed,
                    KeyCode::Space => self.jump = is_pressed,
                    _ => (),
                }
            }
        }
    }

    // Called every frame at fixed rate of 60 FPS.
    fn on_update(&mut self, context: &mut ScriptContext) {
        // The script can be assigned to any scene node, but we assert that it will work only with
        // 2d rigid body nodes.
        if let Some(rigid_body) = context.scene.graph[context.handle].cast_mut::<RigidBody>() {
            let x_speed = if self.move_left {
                3.0
            } else if self.move_right {
                -3.0
            } else {
                0.0
            };

            if x_speed != 0.0 {
                self.current_animation = 0;
            } else {
                self.current_animation = 1;
            }

            if self.jump {
                rigid_body.set_lin_vel(Vector2::new(x_speed, 4.0))
            } else {
                rigid_body.set_lin_vel(Vector2::new(x_speed, rigid_body.lin_vel().y))
            };

            // It is always a good practice to check whether the handles are valid, at this point we don't know
            // for sure what's the value of the `sprite` field. It can be unassigned and the following code won't
            // execute. A simple `context.scene.graph[self.sprite]` would just panicked in this case.
            if let Some(sprite) = context.scene.graph.try_get_mut(self.sprite) {
                // We want to change player orientation only if he's moving.
                if x_speed != 0.0 {
                    let local_transform = sprite.local_transform_mut();

                    let current_scale = **local_transform.scale();

                    local_transform.set_scale(Vector3::new(
                        // Just change X scaling to mirror player's sprite.
                        current_scale.x.copysign(-x_speed),
                        current_scale.y,
                        current_scale.z,
                    ));
                }
            }
        }

        if let Some(current_animation) = self.animations.get_mut(self.current_animation as usize) {
            current_animation.update(context.dt);

            if let Some(sprite) = context
                .scene
                .graph
                .try_get_mut(self.sprite)
                .and_then(|n| n.cast_mut::<Rectangle>())
            {
                // Set new frame to the sprite.
                sprite.set_texture(current_animation.texture());
                sprite.set_uv_rect(
                    current_animation
                        .current_frame_uv_rect()
                        .unwrap_or_default(),
                );
            }
        }
    }

    // Returns unique script id for serialization needs.
    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}
