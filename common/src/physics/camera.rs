//! This module contains the definition of the `Camera`s.
//!
//! A `Camera` defines how a player's entity reacts to that player's inputs.

use crate::{
    debug::send_debug_info, physics::player::PhysicsPlayer, player::PlayerInput,
};
use super::BlockContainer;
use nalgebra::{Vector2, Vector3};

// Unit vector in the `angle` direction
fn movement_direction(yaw: f64, angle: f64) -> Vector3<f64> {
    let yaw = yaw + angle;
    Vector3::new(-yaw.to_radians().sin(), 0.0, -yaw.to_radians().cos()).normalize()
}
// Normalize the vector if it can be normalized or return 0 othersize
fn normalize_or_zero(v: Vector3<f64>) -> Vector3<f64> {
    if v.norm() > 1e-9f64 {
        v.normalize()
    } else {
        Vector3::zeros()
    }
}

#[derive(Default, Clone, Copy)]
struct State {
    position: Vector3<f64>,
    velocity: Vector3<f64>
}

#[derive(Default, Clone, Copy)]
struct Derivative {
    velocity: Vector3<f64>,
    acceleration: Vector3<f64>
}

fn evaluate<AccelF>(initial_state: &State, t: f64, dt: f64, d: Option<Derivative>, acceleration: AccelF) -> Derivative
where AccelF: Fn(&State, f64) -> Vector3<f64> {
    let d = d.unwrap_or_else(Derivative::default);
    let state = State {
        position: initial_state.position + d.velocity*dt,
        velocity: initial_state.velocity + d.acceleration*dt,
    };

    Derivative {
        velocity: state.velocity,
        acceleration: acceleration(&state, t + dt)
    }
}

fn integrate<AccelF>(state: &mut State, t: f64, dt: f64, acceleration: &AccelF)
where AccelF: Fn(&State, f64) -> Vector3<f64> {
    let a = evaluate(&state, t, 0.0, None, acceleration);
    let b = evaluate(&state, t, dt * 0.5, Some(a), acceleration);
    let c = evaluate(&state, t, dt * 0.5, Some(b), acceleration);
    let d = evaluate(&state, t, dt, Some(c), acceleration);

    let dxdt = 1.0 / 6.0 *
        (a.velocity + 2.0 * (b.velocity + c.velocity) + d.velocity);

    let dvdt = 1.0 / 6.0 *
        (a.acceleration + 2.0 * (b.acceleration + c.acceleration) + d.acceleration);

    state.position += dxdt * dt;
    state.velocity += dvdt * dt;
}

trait PlayerCamera {
    const ACCELERATION: f64;
    const MAX_SPEED: f64;

    fn compute_movement<BC: BlockContainer>(
        player: &mut PhysicsPlayer,
        input: PlayerInput,
        seconds_delta: f64,
        world: &BC,
    );
}

pub struct FlyingCamera;

impl PlayerCamera for FlyingCamera {
    const ACCELERATION: f64 = 8.0;
    const MAX_SPEED: f64 = 30.0;

    fn compute_movement<BC: BlockContainer>(player: &mut PhysicsPlayer, input: PlayerInput, seconds_delta: f64, world: &BC) {
        // We're flying, so reset Y velocity to zero.
        player.velocity.y = 0.0;
        
        // Calculate the intended acceleration based on controls.
        let mut player_acceleration = Vector3::zeros();
        if input.key_move_forward {
            player_acceleration += movement_direction(input.yaw, 0.0);
        }
        if input.key_move_left {
            player_acceleration += movement_direction(input.yaw, 90.0);
        }
        if input.key_move_backward {
            player_acceleration += movement_direction(input.yaw, 180.0);
        }
        if input.key_move_right {
            player_acceleration += movement_direction(input.yaw, 270.0);
        }

        // Use RK4 integration to estimate the movement from the time delta.
        let mut state = State {
            position: player.position().coords,
            velocity: player.velocity
        };

        integrate(&mut state, 1.0/seconds_delta, seconds_delta, &move |state, _| {
            // -Self::ACCELERATION * state.position - player_acceleration + state.velocity
            player_acceleration * Self::ACCELERATION
        });

        let mut expected_movement = state.velocity;
        if expected_movement.norm() > Self::MAX_SPEED {
            expected_movement *= Self::MAX_SPEED / expected_movement.norm();
        }

        if input.key_move_up {
            expected_movement.y += (seconds_delta * Self::MAX_SPEED) as f64;
        }
        if input.key_move_down {
            expected_movement.y -= (seconds_delta * Self::MAX_SPEED) as f64;
        }

        player.velocity = player.move_check_collision(world, expected_movement);
    }
}

/// The default camera. It doesn't let you go inside blocks unless you are already inside blocks.
// TODO: use better integrator (RK4 ?)
pub fn default_camera<BC: BlockContainer>(
    player: &mut PhysicsPlayer,
    input: PlayerInput,
    seconds_delta: f64,
    world: &BC,
) {
    // Compute the expected movement of the player, i.e. assuming there are no collisions.
    if input.flying || player.intersect_world(world) {
        FlyingCamera::compute_movement(player, input, seconds_delta, world)
    } else {
        // Not flying
        const JUMP_SPEED: f64 = 8.0;
        const GRAVITY_ACCELERATION: f64 = 25.0;
        const MAX_DOWN_SPEED: f64 = 30.0;
        const HORIZONTAL_SPEED: f64 = 7.0;
        player.velocity.x = 0.0;
        player.velocity.z = 0.0;
        let mut horizontal_velocity = Vector3::zeros();
        if input.key_move_forward {
            horizontal_velocity += movement_direction(input.yaw, 0.0);
        }
        if input.key_move_left {
            horizontal_velocity += movement_direction(input.yaw, 90.0);
        }
        if input.key_move_backward {
            horizontal_velocity += movement_direction(input.yaw, 180.0);
        }
        if input.key_move_right {
            horizontal_velocity += movement_direction(input.yaw, 270.0);
        }
        let horizontal_velocity = normalize_or_zero(horizontal_velocity) * HORIZONTAL_SPEED;
        if player.is_on_ground(world) {
            player.velocity.y = if input.key_move_up { JUMP_SPEED } else { 0.0 };
        } else {
            player.velocity.y -= GRAVITY_ACCELERATION * seconds_delta;
            if player.velocity.y < -MAX_DOWN_SPEED {
                player.velocity.y = -MAX_DOWN_SPEED;
            }
        };
        let expected_movement = (player.velocity + horizontal_velocity) * seconds_delta;
        player.move_check_collision(world, expected_movement);
    }
    // TODO: add a noclip camera mode
    send_debug_info(
        "Physics",
        "ontheground",
        format!(
            "Player 0 on the ground? {}",
            player.is_on_ground(world)
        ),
    );
    let [vx, vy, vz]: [f64; 3] = player.velocity.into();
    send_debug_info(
        "Physics",
        "velocity",
        format!("velocity: {:.2} {:.2} {:.2}", vx, vy, vz),
    );
}
