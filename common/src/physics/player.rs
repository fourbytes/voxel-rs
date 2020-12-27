use nalgebra::{Point3, Vector3, Isometry3};
use ncollide3d::bounding_volume::AABB;

use crate::world::BlockPos;
use super::BlockContainer;

const PLAYER_SIDE: f64 = 0.8;
const PLAYER_HEIGHT: f64 = 1.8;
const CAMERA_OFFSET: [f64; 3] = [0.0, 1.6, 0.0];

fn aabb_intersects_world<BC: BlockContainer>(world: &BC, aabb: &AABB<f64>) -> bool {
    let mins = aabb.mins.map(|c| c.floor() as i64);
    let maxs = aabb.maxs.map(|c| c.ceil() as i64);

    for i in mins.x..maxs.x {
        for j in mins.y..maxs.y {
            for k in mins.z..maxs.z {
                if world.is_block_full((i, j, k).into()) {
                    return true;
                }
            }
        }
    }
    return false;
}

/// The physics representation of a player
#[derive(Debug, Clone)]
pub struct PhysicsPlayer {
    /// The aabb of the player
    pub aabb: AABB<f64>,
    /// The current velocity of the player
    pub velocity: Vector3<f64>,
}

impl PhysicsPlayer {
    pub fn from_coords(coords: Point3<f64>) -> Self {
        Self {
            aabb: AABB::from_half_extents(
                coords,
                Vector3::new(PLAYER_SIDE, PLAYER_HEIGHT, PLAYER_SIDE)),
            velocity: Vector3::zeros()
        }
    }

    /// Try to move the box in the world and stop the movement if it goes trough a block
    /// Return the actual deplacement
    pub fn move_check_collision<BC: BlockContainer>(&mut self, world: &BC, delta: Vector3<f64>) -> Vector3<f64> {
        if self.intersect_world(world) {
            self.aabb = self.aabb.transform_by(&Isometry3::new(delta, Vector3::zeros()));
            return delta;
        }

        // How many blocks are we moving?
        let step = delta.zip_map(&self.aabb.extents(), |d, s| {
            (d.abs() / s).ceil() as u32
        });
        let dd = delta.zip_map(&step, |d, s| {
            d / (s as f64)
        });

        let old_pos = self.aabb;

        // Loop the X, Y, and Z dimension.
        for r in 0..3 {
            let mut dimension_delta = Vector3::zeros();
            dimension_delta[r] = dd[r];
            let mut new_pos = self.aabb;

            for _ in 0..step[r] {
                let mut should_break = false;
                new_pos = new_pos.transform_by(&Isometry3::new(dimension_delta, Vector3::zeros()));
                if aabb_intersects_world(world, &new_pos) {
                    new_pos = new_pos.transform_by(&Isometry3::new(-dimension_delta, Vector3::zeros()));

                    let mut min_d = 0.0;
                    let mut max_d = dd[r].abs();

                    while max_d - min_d > 0.001 {
                        // binary search the max delta
                        let med = (min_d + max_d) / 2.0;
                        let mut delta_d = Vector3::zeros();
                        delta_d[r] = med * dd[r].signum();
                        let pot_pos = new_pos.transform_by(&Isometry3::new(delta_d, Vector3::zeros()));
                        if aabb_intersects_world(world, &pot_pos) {
                            max_d = med;
                        } else {
                            min_d = med;
                        }
                    }

                    let mut delta_d = Vector3::zeros();
                    delta_d[r] = dd[r].signum() * min_d / 2.0;
                    new_pos = new_pos.transform_by(&Isometry3::new(delta_d, Vector3::zeros()));
                    should_break = true
                }

                self.aabb = new_pos;

                if should_break {
                    break
                }
            }
        }

        self.aabb.mins - old_pos.mins
    }
    
    /// Check if player is on ground in world.
    pub fn is_on_ground<BC: BlockContainer>(&self, world: &BC) -> bool {
        let new_bounds = self.aabb.transform_by(&Isometry3::new(Vector3::new(0.0, -0.0021, 0.0), Vector3::zeros()));
        let would_intersect_down = aabb_intersects_world(world, &new_bounds);
        !self.intersect_world(world) && would_intersect_down
    }

    /// Check if player is intersecting with the world.
    pub fn intersect_world<BC: BlockContainer>(&self, world: &BC) -> bool {
        return aabb_intersects_world(world, &self.aabb);
    }

    /// Get the coords of the player.
    pub fn position(&self) -> Point3<f64> {
        let mut c = self.aabb.center();
        c.coords.y = self.aabb.mins.coords.y;
        c
    }

    /// Get the position of the camera
    pub fn get_camera_position(&self) -> Point3<f64> {
        self.position() + Vector3::from(CAMERA_OFFSET)
    }

    /// Ray trace to find the pointed block. Return the position of the block and the face (x/-x/y/-y/z/-z)
    // TODO: use block registry
    pub fn get_pointed_at<BC: BlockContainer>(
        &self,
        dir: Vector3<f64>,
        mut max_dist: f64,
        world: &BC,
    ) -> Option<(BlockPos, usize)> {
        let dir = dir.normalize();
        let mut pos = self.get_camera_position();

        // Check current block first
        let was_inside = world.is_block_full(BlockPos::from(pos));
        let dirs = [
            Vector3::new(-1.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, -1.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, -1.0),
            Vector3::new(0.0, 0.0, 1.0),
        ];
        loop {
            let targets = [
                pos.x.floor(),
                pos.x.ceil(),
                pos.y.floor(),
                pos.y.ceil(),
                pos.z.floor(),
                pos.z.ceil(),
            ];

            let mut curr_min = 1e9;
            let mut face = 0;

            for i in 0..6 {
                let effective_movement = dir.dot(&dirs[i]);
                if effective_movement > 1e-6 {
                    let dir_offset = (targets[i].abs() - pos.coords.dot(&dirs[i]).abs()).abs();
                    let dist = dir_offset / effective_movement;
                    if curr_min > dist {
                        curr_min = dist;
                        face = i;
                    }
                }
            }

            if was_inside {
                return Some((BlockPos::from(pos), face ^ 1));
            }

            if curr_min > max_dist {
                return None;
            } else {
                curr_min += 1e-5;
                max_dist -= curr_min;
                pos += curr_min * dir;
                let block_pos = BlockPos::from(pos);
                if world.is_block_full(block_pos) {
                    return Some((block_pos, face));
                }
            }
        }
    }
}

impl Default for PhysicsPlayer {
    fn default() -> Self {
        Self {
            aabb: AABB::from_half_extents(
                Point3::new(1.46, 58.6, 1.85),
                Vector3::new(PLAYER_SIDE / 2.0, PLAYER_HEIGHT / 2.0, PLAYER_SIDE / 2.0),
            ),
            velocity: Vector3::zeros(),
        }
    }
}
