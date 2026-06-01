use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;

// ─── Constants ───────────────────────────────────────────────────────────────

const WINDOW_WIDTH: f32 = 800.0;
const WINDOW_HEIGHT: f32 = 600.0;

const CAPYBARA_WIDTH: f32 = 120.0;
const CAPYBARA_HEIGHT: f32 = 40.0;
const CAPYBARA_SPEED: f32 = 400.0;
const CAPYBARA_Y: f32 = -200.0;

const PASSENGER_SIZE: f32 = 28.0;
const PASSENGER_SPAWN_INTERVAL: f32 = 1.8;
const PASSENGER_FALL_SPEED: f32 = -120.0;

const MAX_PASSENGERS: u32 = 8; // stack too high = topple!
const TOPPLE_ANGLE_DEG: f32 = 35.0; // degrees tilt before game over

const RIVER_SCROLL_SPEED: f32 = 60.0;

// ─── Components ──────────────────────────────────────────────────────────────

#[derive(Component)]
struct Capybara {
    tilt: f32,        // accumulated lean in degrees
    tilt_velocity: f32,
}

#[derive(Component)]
struct Passenger {
    kind: PassengerKind,
    landed: bool,
    stack_position: Option<u32>, // which slot on the capybara's back
}

#[derive(Component)]
struct Balanced; // tag: successfully riding on the capybara

#[derive(Component)]
struct RiverTile {
    scroll_x: f32,
}

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct TiltBar;

#[derive(Component)]
struct GameOverScreen;

#[derive(Clone, Copy, PartialEq)]
enum PassengerKind {
    Duckling,
    Frog,
    Butterfly, // bonus: lightweight, reduces tilt
}

impl PassengerKind {
    fn weight(&self) -> f32 {
        match self {
            PassengerKind::Duckling => 1.0,
            PassengerKind::Frog => 1.8,
            PassengerKind::Butterfly => -0.4, // negative = balancing help
        }
    }

    fn color(&self) -> Color {
        match self {
            PassengerKind::Duckling => Color::srgb(1.0, 0.85, 0.1),
            PassengerKind::Frog => Color::srgb(0.2, 0.75, 0.3),
            PassengerKind::Butterfly => Color::srgb(0.8, 0.3, 0.9),
        }
    }

    fn emoji_char(&self) -> &'static str {
        match self {
            PassengerKind::Duckling => "🐥",
            PassengerKind::Frog => "🐸",
            PassengerKind::Butterfly => "🦋",
        }
    }

    fn score_value(&self) -> u32 {
        match self {
            PassengerKind::Duckling => 10,
            PassengerKind::Frog => 20,
            PassengerKind::Butterfly => 5,
        }
    }
}

// ─── Resources ───────────────────────────────────────────────────────────────

#[derive(Resource)]
struct GameState {
    score: u32,
    passengers_on_board: u32,
    spawn_timer: Timer,
    game_over: bool,
    difficulty_timer: f32,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            score: 0,
            passengers_on_board: 0,
            spawn_timer: Timer::from_seconds(PASSENGER_SPAWN_INTERVAL, TimerMode::Repeating),
            game_over: false,
            difficulty_timer: 0.0,
        }
    }
}

// ─── States ──────────────────────────────────────────────────────────────────

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum AppState {
    #[default]
    Playing,
    GameOver,
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "🦫 Capybara Commute".into(),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        // Uncomment the next line for physics debug outlines:
        // .add_plugins(RapierDebugRenderPlugin::default())
        .init_state::<AppState>()
        .init_resource::<GameState>()
        .add_systems(Startup, (setup_camera, setup_river, setup_capybara, setup_ui))
        .add_systems(
            Update,
            (
                move_capybara,
                spawn_passengers,
                move_falling_passengers,
                land_passengers,
                update_balanced_passengers,
                update_tilt,
                scroll_river,
                update_ui,
                check_game_over,
            )
                .run_if(in_state(AppState::Playing)),
        )
        .add_systems(OnEnter(AppState::GameOver), show_game_over)
        .add_systems(
            Update,
            restart_game.run_if(in_state(AppState::GameOver)),
        )
        .run();
}

// ─── Setup Systems ────────────────────────────────────────────────────────────

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn setup_river(mut commands: Commands) {
    // Tiled river background — two wide strips that scroll left
    for i in 0..3 {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(0.18, 0.55, 0.78),
                    custom_size: Some(Vec2::new(WINDOW_WIDTH + 10.0, WINDOW_HEIGHT)),
                    ..default()
                },
                transform: Transform::from_xyz(
                    i as f32 * WINDOW_WIDTH,
                    0.0,
                    -10.0,
                ),
                ..default()
            },
            RiverTile { scroll_x: i as f32 * WINDOW_WIDTH },
        ));
    }

    // Decorative water ripple strips
    for row in [-150_f32, -50.0, 50.0, 150.0] {
        for i in 0..5 {
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgba(1.0, 1.0, 1.0, 0.07),
                        custom_size: Some(Vec2::new(120.0, 8.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        -400.0 + i as f32 * 200.0,
                        row,
                        -9.0,
                    ),
                    ..default()
                },
                RiverTile { scroll_x: -400.0 + i as f32 * 200.0 },
            ));
        }
    }
}

fn setup_capybara(mut commands: Commands) {
    // The capybara body (platform)
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.6, 0.42, 0.25),
                custom_size: Some(Vec2::new(CAPYBARA_WIDTH, CAPYBARA_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, CAPYBARA_Y, 0.0),
            ..default()
        },
        Capybara { tilt: 0.0, tilt_velocity: 0.0 },
        RigidBody::KinematicPositionBased,
        Collider::cuboid(CAPYBARA_WIDTH / 2.0, CAPYBARA_HEIGHT / 2.0),
        KinematicCharacterController::default(),
    ));

    // Eyes (decorative children handled as separate sprites)
    // Left eye
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::srgb(0.1, 0.05, 0.0),
            custom_size: Some(Vec2::new(8.0, 8.0)),
            ..default()
        },
        transform: Transform::from_xyz(-25.0, CAPYBARA_Y + 16.0, 1.0),
        ..default()
    });
    // Right eye
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::srgb(0.1, 0.05, 0.0),
            custom_size: Some(Vec2::new(8.0, 8.0)),
            ..default()
        },
        transform: Transform::from_xyz(25.0, CAPYBARA_Y + 16.0, 1.0),
        ..default()
    });
    // Nose
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::srgb(0.5, 0.25, 0.18),
            custom_size: Some(Vec2::new(22.0, 14.0)),
            ..default()
        },
        transform: Transform::from_xyz(42.0, CAPYBARA_Y + 6.0, 1.0),
        ..default()
    });
}

fn setup_ui(mut commands: Commands) {
    // Score
    commands.spawn((
        TextBundle::from_section(
            "Score: 0",
            TextStyle {
                font_size: 28.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(16.0),
            ..default()
        }),
        ScoreText,
    ));

    // Tilt bar label
    commands.spawn(
        TextBundle::from_section(
            "Balance:",
            TextStyle {
                font_size: 20.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            right: Val::Px(180.0),
            ..default()
        }),
    );

    // Tilt bar background
    commands.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            top: Val::Px(14.0),
            right: Val::Px(16.0),
            width: Val::Px(150.0),
            height: Val::Px(22.0),
            ..default()
        },
        background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.4)),
        ..default()
    });

    // Tilt bar fill
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(14.0),
                right: Val::Px(16.0),
                width: Val::Px(0.0), // updated each frame
                height: Val::Px(22.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgb(0.2, 0.9, 0.3)),
            ..default()
        },
        TiltBar,
    ));

    // Controls hint
    commands.spawn(
        TextBundle::from_section(
            "← → to move   Balance your passengers!",
            TextStyle {
                font_size: 16.0,
                color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            ..default()
        }),
    );
}

// ─── Gameplay Systems ─────────────────────────────────────────────────────────

fn move_capybara(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut KinematicCharacterController, &Transform), With<Capybara>>,
) {
    let Ok((mut controller, transform)) = query.get_single_mut() else { return };

    let mut direction = 0.0_f32;
    if keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA) {
        direction -= 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD) {
        direction += 1.0;
    }

    let delta = direction * CAPYBARA_SPEED * time.delta_seconds();
    let new_x = (transform.translation.x + delta)
        .clamp(-WINDOW_WIDTH / 2.0 + CAPYBARA_WIDTH / 2.0, WINDOW_WIDTH / 2.0 - CAPYBARA_WIDTH / 2.0);

    controller.translation = Some(Vec2::new(new_x - transform.translation.x, 0.0));
}

fn spawn_passengers(
    mut commands: Commands,
    mut game: ResMut<GameState>,
    time: Res<Time>,
) {
    if game.game_over { return; }

    game.difficulty_timer += time.delta_seconds();
    // Speed up spawning over time (min 0.7s interval)
    let interval = (PASSENGER_SPAWN_INTERVAL - game.difficulty_timer * 0.02).max(0.7);
    game.spawn_timer.set_duration(std::time::Duration::from_secs_f32(interval));

    if !game.spawn_timer.tick(time.delta()).just_finished() {
        return;
    }

    let mut rng = rand::thread_rng();
    let x = rng.gen_range(-340.0_f32..340.0);

    // Weighted random: 50% duckling, 35% frog, 15% butterfly
    let roll: f32 = rng.gen();
    let kind = if roll < 0.50 {
        PassengerKind::Duckling
    } else if roll < 0.85 {
        PassengerKind::Frog
    } else {
        PassengerKind::Butterfly
    };

    let spawn_y = WINDOW_HEIGHT / 2.0 + PASSENGER_SIZE;

    commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: kind.color(),
                    custom_size: Some(Vec2::new(PASSENGER_SIZE, PASSENGER_SIZE)),
                    ..default()
                },
                transform: Transform::from_xyz(x, spawn_y, 1.0),
                ..default()
            },
            Passenger { kind, landed: false, stack_position: None },
            RigidBody::Dynamic,
            Collider::ball(PASSENGER_SIZE / 2.0),
            GravityScale(0.0), // we control fall manually for better feel
            Velocity::default(),
            Restitution::coefficient(0.2),
            Damping { linear_damping: 2.0, angular_damping: 5.0 },
        ))
        .with_children(|parent| {
            // White outline border
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::WHITE,
                    custom_size: Some(Vec2::new(PASSENGER_SIZE + 5.0, PASSENGER_SIZE + 5.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, -0.1),
                ..default()
            });
            // Left eye
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(0.05, 0.05, 0.1),
                    custom_size: Some(Vec2::new(5.0, 5.0)),
                    ..default()
                },
                transform: Transform::from_xyz(-5.0, 4.0, 0.2),
                ..default()
            });
            // Right eye
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(0.05, 0.05, 0.1),
                    custom_size: Some(Vec2::new(5.0, 5.0)),
                    ..default()
                },
                transform: Transform::from_xyz(5.0, 4.0, 0.2),
                ..default()
            });
            // Type-specific detail
            match kind {
                PassengerKind::Duckling => {
                    // Orange beak
                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            color: Color::srgb(1.0, 0.5, 0.0),
                            custom_size: Some(Vec2::new(9.0, 5.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(0.0, -3.0, 0.2),
                        ..default()
                    });
                }
                PassengerKind::Frog => {
                    // Bulgy eye rings (lighter green circles behind the eyes)
                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            color: Color::srgb(0.4, 0.95, 0.5),
                            custom_size: Some(Vec2::new(9.0, 9.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(-5.0, 5.0, 0.1),
                        ..default()
                    });
                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            color: Color::srgb(0.4, 0.95, 0.5),
                            custom_size: Some(Vec2::new(9.0, 9.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(5.0, 5.0, 0.1),
                        ..default()
                    });
                }
                PassengerKind::Butterfly => {
                    // Left wing
                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            color: Color::srgba(0.95, 0.55, 1.0, 0.85),
                            custom_size: Some(Vec2::new(18.0, 12.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(-18.0, 3.0, -0.05),
                        ..default()
                    });
                    // Right wing
                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            color: Color::srgba(0.95, 0.55, 1.0, 0.85),
                            custom_size: Some(Vec2::new(18.0, 12.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(18.0, 3.0, -0.05),
                        ..default()
                    });
                }
            }
        });
}

fn move_falling_passengers(
    mut query: Query<(&mut Velocity, &Transform, &Passenger)>,
) {
    for (mut vel, _transform, passenger) in query.iter_mut() {
        if !passenger.landed {
            vel.linvel.y = PASSENGER_FALL_SPEED;
        }
    }
}

fn land_passengers(
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut passengers: Query<(Entity, &mut Passenger, &Transform, &mut Velocity)>,
    capybara: Query<&Transform, With<Capybara>>,
) {
    let Ok(cap_transform) = capybara.get_single() else { return };
    let cap_x = cap_transform.translation.x;
    let cap_y = cap_transform.translation.y;

    // The "catch line" is the top surface of the capybara
    let catch_line = cap_y + CAPYBARA_HEIGHT / 2.0;

    for (entity, mut passenger, p_transform, mut vel) in passengers.iter_mut() {
        if passenger.landed { continue; }

        let px = p_transform.translation.x;
        let py = p_transform.translation.y;

        // Very generous X: anywhere over the capybara body
        let over_capybara = (px - cap_x).abs() < CAPYBARA_WIDTH / 2.0 + 10.0;

        // Tall catch window so fast passengers don't slip through
        let at_catch_height = py <= catch_line + PASSENGER_SIZE && py >= catch_line - 40.0;

        if over_capybara && at_catch_height {
            passenger.landed = true;
            passenger.stack_position = Some(game.passengers_on_board);
            game.passengers_on_board += 1;
            game.score += passenger.kind.score_value();

            vel.linvel = Vec2::ZERO;
            vel.angvel = 0.0;

            commands.entity(entity).insert(Balanced);
        }

        // Fell off screen — despawn
        if py < -WINDOW_HEIGHT / 2.0 - 50.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn update_balanced_passengers(
    capybara: Query<&Transform, With<Capybara>>,
    mut passengers: Query<(&Passenger, &mut Transform), (With<Balanced>, Without<Capybara>)>,
) {
    let Ok(cap_t) = capybara.get_single() else { return };
    let passenger_count = passengers.iter().count() as f32;

    for (passenger, mut p_transform) in passengers.iter_mut() {
        if let Some(slot) = passenger.stack_position {
            // Stack them on the capybara's back
            let stack_y = cap_t.translation.y
                + CAPYBARA_HEIGHT / 2.0
                + PASSENGER_SIZE / 2.0
                + slot as f32 * (PASSENGER_SIZE + 2.0);

            // Center the group of landed passengers and spread them evenly.
            let offset_x = if passenger_count > 0.0 {
                (slot as f32 - (passenger_count - 1.0) * 0.5) * 12.0
            } else {
                0.0
            };

            p_transform.translation.x = cap_t.translation.x + offset_x;
            p_transform.translation.y = stack_y;
        }
    }
}

fn update_tilt(
    mut capybara: Query<(&mut Capybara, &mut Transform)>,
    passengers: Query<(&Passenger, &Transform), (With<Balanced>, Without<Capybara>)>,
    time: Res<Time>,
) {
    let Ok((mut cap, mut cap_transform)) = capybara.get_single_mut() else { return };

    // Calculate center of mass of all passengers relative to capybara center
    let mut total_torque = 0.0_f32;
    let mut total_weight = 0.0_f32;
    let cap_x = cap_transform.translation.x;

    for (passenger, p_transform) in passengers.iter() {
        let offset = p_transform.translation.x - cap_x;
        let weight = passenger.kind.weight();
        total_torque += offset * weight;
        total_weight += weight.abs();
    }

    // Tilt physics: torque from offset weight
    let tilt_force = if total_weight > 0.0 {
        total_torque / (total_weight.max(1.0)) * 0.8
    } else {
        0.0
    };

    cap.tilt_velocity += tilt_force * time.delta_seconds() * 3.0;
    cap.tilt_velocity *= 0.92; // damping
    cap.tilt += cap.tilt_velocity * time.delta_seconds() * 40.0;

    // Clamp tilt to ±40 degrees
    cap.tilt = cap.tilt.clamp(-40.0, 40.0);

    // Apply visual rotation
    cap_transform.rotation = Quat::from_rotation_z(-cap.tilt.to_radians());
}

fn scroll_river(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut RiverTile)>,
) {
    for (mut transform, mut tile) in query.iter_mut() {
        tile.scroll_x -= RIVER_SCROLL_SPEED * time.delta_seconds();

        if tile.scroll_x < -WINDOW_WIDTH {
            tile.scroll_x += WINDOW_WIDTH * 2.0;
        }

        transform.translation.x = tile.scroll_x;
    }
}

fn update_ui(
    game: Res<GameState>,
    capybara: Query<&Capybara>,
    mut score_text: Query<&mut Text, With<ScoreText>>,
    mut tilt_bar: Query<&mut Style, With<TiltBar>>,
) {
    // Score
    if let Ok(mut text) = score_text.get_single_mut() {
        text.sections[0].value = format!("Score: {}  🦫×{}", game.score, game.passengers_on_board);
    }

    // Tilt bar
    if let (Ok(cap), Ok(mut bar_style)) = (capybara.get_single(), tilt_bar.get_single_mut()) {
        let tilt_pct = (cap.tilt.abs() / TOPPLE_ANGLE_DEG).min(1.0);
        bar_style.width = Val::Px(150.0 * tilt_pct);
    }
}

fn check_game_over(
    mut game: ResMut<GameState>,
    capybara: Query<&Capybara>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if game.game_over { return; }

    let Ok(cap) = capybara.get_single() else { return };

    let toppled = cap.tilt.abs() >= TOPPLE_ANGLE_DEG;
    let overloaded = game.passengers_on_board >= MAX_PASSENGERS;

    if toppled || overloaded {
        game.game_over = true;
        next_state.set(AppState::GameOver);
    }
}

// ─── Game Over ────────────────────────────────────────────────────────────────

fn show_game_over(
    mut commands: Commands,
    game: Res<GameState>,
) {
    // Dim overlay
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
            ..default()
        },
        GameOverScreen,
    ))
    .with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            "🦫 Oh no! Everyone fell off!",
            TextStyle {
                font_size: 36.0,
                color: Color::WHITE,
                ..default()
            },
        ));
        parent.spawn(TextBundle::from_section(
            format!("Final Score: {}", game.score),
            TextStyle {
                font_size: 28.0,
                color: Color::srgb(1.0, 0.9, 0.2),
                ..default()
            },
        ));
        parent.spawn(TextBundle::from_section(
            "Press R to try again",
            TextStyle {
                font_size: 22.0,
                color: Color::srgba(1.0, 1.0, 1.0, 0.85),
                ..default()
            },
        ));
    });
}

fn restart_game(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut game: ResMut<GameState>,
    game_over_ui: Query<Entity, With<GameOverScreen>>,
    passengers: Query<Entity, With<Passenger>>,
    mut capybara: Query<(&mut Transform, &mut Capybara)>,
) {
    if !keyboard.just_pressed(KeyCode::KeyR) { return; }

    // Remove game over screen
    for entity in game_over_ui.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Remove all passengers
    for entity in passengers.iter() {
        commands.entity(entity).despawn();
    }

    // Reset capybara
    if let Ok((mut t, mut cap)) = capybara.get_single_mut() {
        t.translation.x = 0.0;
        t.rotation = Quat::IDENTITY;
        cap.tilt = 0.0;
        cap.tilt_velocity = 0.0;
    }

    // Reset game state
    *game = GameState::default();

    next_state.set(AppState::Playing);
}
