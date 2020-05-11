use crate::light::{ChunkLightingWorker, ChunkLightingData};
use crate::worldgen::{WorldGenerationWorker, WorldGenerationState};
use anyhow::Result;
use nalgebra::Vector3;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use voxel_rs_common::light::ChunkLightingState;
use voxel_rs_common::physics::aabb::AABB;
use voxel_rs_common::physics::player::PhysicsPlayer;
use voxel_rs_common::{
    data::{load_data, Data},
    debug::{send_debug_info, send_perf_breakdown},
    network::{
        messages::{ToClient, ToServer},
        ServerIO, ServerEvent,
    },
    physics::simulation::ServerPhysicsSimulation,
    player::PlayerId,
    world::{
        chunk::{ ChunkPos, ChunkPosXZ, Chunk },
        BlockPos, World,
    },
    worldgen::DefaultWorldGenerator,
};
use voxel_rs_common::world::HighestOpaqueBlock;
use voxel_rs_common::time::BreakdownCounter;
use super::{ D, PlayerData };


pub struct Server {
    io: Box<dyn ServerIO>,
    timing: BreakdownCounter,
    game_data: Data,
    worldgen_worker: WorldGenerationWorker,
    light_worker: ChunkLightingWorker,
    physics_simulation: ServerPhysicsSimulation,
    world: Box<World>,
    players: HashMap<PlayerId, PlayerData>,
    generating_chunks: HashSet<ChunkPos>,
    updated_chunks: HashSet<ChunkPos>,
    chunk_lighting_updates: HashSet<ChunkPos>
}

impl Server {
    pub fn new(server_io: Box<dyn ServerIO>) -> Result<Self> {
        log::info!("Initializing server...");
        let game_data = load_data("data".into())?;
        let worldgen_worker = WorldGenerationWorker::new(
            WorldGenerationState::new(
                game_data.blocks.clone(),
                Box::new(DefaultWorldGenerator::new(&game_data.blocks.clone())),
            ),
            "World Generation".to_owned(),
        );
        let light_worker = ChunkLightingWorker::new(ChunkLightingState::new(), "Lighting".to_owned());
        Ok(Self {
            io: server_io,
            timing: BreakdownCounter::new(),
            game_data,
            worldgen_worker,
            light_worker,
            physics_simulation: ServerPhysicsSimulation::new(),
            world: Box::new(World::new()),
            players: HashMap::new(),
            generating_chunks: HashSet::new(),
            updated_chunks: HashSet::new(),
            chunk_lighting_updates: HashSet::new()
        })
    }
    
    pub fn launch(mut self) -> Result<()> {
        log::info!("Starting server...");
        
        loop {
            self.tick();
        }
    }

    fn save_chunk(&mut self, chunk: Chunk) {
        let chunk_pos = chunk.pos;
        self.updated_chunks.insert(chunk_pos);
        self.world.set_chunk(Arc::new(chunk));

        if self.world.update_highest_opaque_block(chunk_pos) {
            // recompute the light of the 3x3 columns
            for &c_pos in self.world.chunks.keys() {
                if c_pos.py <= chunk_pos.py
                    && (c_pos.px - chunk_pos.px).abs() <= 1
                        && (c_pos.pz - chunk_pos.pz).abs() <= 1
                {
                    self.chunk_lighting_updates.insert(c_pos);
                }
            }
        } else {
            // compute only the light for the chunk
            for &c_pos in self.world.chunks.keys() {
                if (c_pos.py - chunk_pos.py).abs() <= 1
                    && (c_pos.px - chunk_pos.px).abs() <= 1
                        && (c_pos.pz - chunk_pos.pz).abs() <= 1
                {
                    self.chunk_lighting_updates.insert(c_pos);
                }
            }
        }
    }

    fn tick(&mut self) {
        self.updated_chunks = HashSet::new();
        self.timing.start_frame();
        
        // Handle messages
        loop {
            let event = self.io.receive_event();

            // TODO: Remove this, debug logging.
            match &event {
                ServerEvent::NoEvent => (),
                ServerEvent::ClientMessage(id, message) => match message {
                    ToServer::UpdateInput(_) => (),
                    _ => log::trace!("Received client message (id: {:?}): {:?}", id, message)
                },
                _ => log::trace!("Received event: {:?}", event)
            }

            match event {
                ServerEvent::NoEvent => break,
                ServerEvent::ClientConnected(id) => {
                    log::info!("Client connected to the server!");
                    self.physics_simulation.set_player_input(id, Default::default());
                    self.players.insert(id, PlayerData::default());
                    self.io.send(id, ToClient::GameData(self.game_data.clone())); self.io.send(id, ToClient::CurrentId(id));
                }
                ServerEvent::ClientDisconnected(id) => {
                    self.physics_simulation.remove(id);
                    self.players.remove(&id);
                }
                ServerEvent::ClientMessage(id, message) => match message {
                    ToServer::UpdateInput(input) => {
                        assert!(self.players.contains_key(&id));
                        self.physics_simulation.set_player_input(id, input);
                    }
                    ToServer::SetRenderDistance(render_distance) => {
                        assert!(self.players.contains_key(&id));
                        self.players.entry(id).and_modify(move |player_data| {
                            player_data.render_distance = render_distance
                        });
                    }
                    ToServer::BreakBlock(player_pos, yaw, pitch) => {
                        // TODO: check player pos and block
                        let physics_player = PhysicsPlayer {
                            aabb: AABB {
                                pos: player_pos,
                                size_x: 0.0,
                                size_y: 0.0,
                                size_z: 0.0,
                            },
                            velocity: Vector3::zeros(),
                        };
                        if let Some((block_pos, _face)) = physics_player.selected_block(&self.world, yaw, pitch) {
                            if let Some(new_chunk) = self.world.set_block(block_pos, None) {
                                self.save_chunk(new_chunk);
                            }
                        }
                    }
                    ToServer::SelectBlock(player_pos, yaw, pitch) => {
                        // TODO: check player pos and block
                        let physics_player = PhysicsPlayer {
                            aabb: AABB {
                                pos: player_pos,
                                size_x: 0.0,
                                size_y: 0.0,
                                size_z: 0.0,
                            },
                            velocity: Vector3::zeros(),
                        };
                        let y = yaw.to_radians();
                        let p = pitch.to_radians();
                        let dir = Vector3::new(-y.sin() * p.cos(), p.sin(), -y.cos() * p.cos());
                        // TODO: don't hardcode max dist
                        if let Some((block, _face)) =
                            physics_player.get_pointed_at(dir, 10.0, &self.world)
                        {
                            // TODO: careful with more complicated blocks
                            self.players.get_mut(&id).unwrap().block_to_place = self.world.get_block(block);
                        }
                    }
                    ToServer::PlaceBlock(player_pos, yaw, pitch) => {
                        // TODO: check player pos and block
                        let physics_player = PhysicsPlayer {
                            aabb: AABB {
                                pos: player_pos,
                                size_x: 0.0,
                                size_y: 0.0,
                                size_z: 0.0,
                            },
                            velocity: Vector3::zeros(),
                        };
                        let y = yaw.to_radians();
                        let p = pitch.to_radians();
                        let dir = Vector3::new(-y.sin() * p.cos(), p.sin(), -y.cos() * p.cos());
                        // TODO: don't hardcode max dist
                        if let Some((mut block, face)) =
                            physics_player.get_pointed_at(dir, 10.0, &self.world)
                        {
                            block.px += D[face][0];
                            block.py += D[face][1];
                            block.pz += D[face][2];
                            let chunk_pos = block.containing_chunk_pos();
                            if self.world.has_chunk(chunk_pos) {
                                let mut new_chunk = (*self.world.get_chunk(chunk_pos).unwrap()).clone();
                                new_chunk.set_block_at(
                                    block.pos_in_containing_chunk(),
                                    self.players.get(&id).unwrap().block_to_place,
                                );
                                self.save_chunk(new_chunk);
                            }
                        }
                    }
                },
            }
        }
        self.timing.record_part("Network events");

        // Receive generated chunks
        for chunk in self.worldgen_worker.get_processed().into_iter() {
            // Only insert the chunk in the world if it was still being generated.
            if self.generating_chunks.remove(&chunk.pos) {
                let pos = chunk.pos.clone();
                self.save_chunk(chunk);
            }
        }
        self.timing.record_part("Receive generated chunks");

        // Receive lighted chunks
        let mut updated_light_chunks = HashMap::new();
        for light_chunk in self.light_worker.get_processed().into_iter() {
            updated_light_chunks.insert(light_chunk.pos, light_chunk.clone());
            self.world.set_light_chunk(light_chunk);
        }
        self.timing.record_part("Receive lighted chunks");

        // Send light updates
        for chunk_pos in self.chunk_lighting_updates.drain() {
            if self.world.has_chunk(chunk_pos) {
                let mut chunks = Vec::with_capacity(27);
                let mut highest_opaque_blocks = Vec::with_capacity(9);

                for i in -1..=1 {
                    for k in -1..=1 {
                        let pos: ChunkPosXZ = chunk_pos.offset(i, 0, k).into();
                        highest_opaque_blocks.push(
                            (*self.world.highest_opaque_block
                                .entry(pos)
                                .or_insert_with(|| Arc::new(HighestOpaqueBlock::new(pos))))
                                .clone(),
                        );
                    }
                }

                for i in -1..=1 {
                    for j in -1..=1 {
                        for k in -1..=1 {
                            let pos = chunk_pos.offset(i, j, k);
                            chunks.push(self.world.get_chunk(pos));
                        }
                    }
                }

                let data = ChunkLightingData { chunks, highest_opaque_blocks };
                self.light_worker.enqueue(chunk_pos, data);
            }
        }
        self.timing.record_part("Send light updates to worker");

        // Tick game
        self.physics_simulation.step_simulation(Instant::now(), &self.world);
        self.timing.record_part("Update physics");

        // Send updates to players
        for (&player, _) in self.players.iter() {
            self.io.send(
                player,
                ToClient::UpdatePhysics((*self.physics_simulation.get_state()).clone()),
            );
        }
        self.timing.record_part("Send physics updates to players");

        // Send chunks to players
        let mut player_positions = Vec::new();
        for (player, data) in self.players.iter_mut() {
            let player_chunk = BlockPos::from(self.physics_simulation
                .get_state()
                .physics_state
                .players
                .get(player)
                .unwrap()
                .get_camera_position()
            ).containing_chunk_pos();
            player_positions.push((player_chunk, data.render_distance));
            // Send new chunks
            for chunk_pos in data.render_distance.iterate_around_player(player_chunk) {
                // The player hasn't received the chunk yet
                if !data.loaded_chunks.contains(&chunk_pos) || self.updated_chunks.contains(&chunk_pos) {
                    if let Some(chunk) = self.world.get_chunk(chunk_pos) {
                        // Send it to the player if it's in the world
                        self.io.send(
                            *player,
                            ToClient::Chunk(
                                chunk.clone(),
                                self.world.get_add_light_chunk(chunk_pos).clone(),
                            ),
                        );
                        data.loaded_chunks.insert(chunk_pos);
                    } else {
                        // Generate the chunk if it's not already generating
                        let actually_inserted = self.generating_chunks.insert(chunk_pos);
                        if actually_inserted {
                            self.worldgen_worker.enqueue(chunk_pos, ());
                        }
                    }
                }

                if let Some(light_chunk) = updated_light_chunks.get(&chunk_pos) {
                    self.io.send(*player, ToClient::LightChunk(light_chunk.clone()))
                }
            }
            // Drop chunks that are too far away
            let render_distance = data.render_distance;
            data.loaded_chunks
                .retain(|chunk_pos| render_distance.is_chunk_visible(player_chunk, *chunk_pos));
        }
        self.timing.record_part("Send chunks to players");

        // Update player positions for worldgen
        let player_pos = player_positions.iter().map(|x| &x.0).cloned().collect::<Vec<_>>();
        self.worldgen_worker.update_player_pos(player_pos.clone());
        // Update player positions for lighting
        self.light_worker.update_player_pos(player_pos);

        // Drop chunks that are far from all players (and update chunk priorities)
        {
            let worldgen_worker = &mut self.worldgen_worker;
            let generating_chunks = &mut self.generating_chunks;
            let chunk_lighting_updates = &mut self.chunk_lighting_updates;
            let World {
                ref mut chunks,
                ref mut light,
                ..
            } = *self.world;

            chunks.retain(|chunk_pos, _| {
                for (player_chunk, render_distance) in player_positions.iter() {
                    if render_distance.is_chunk_visible(*player_chunk, *chunk_pos) {
                        return true;
                    }
                }
                light.remove(chunk_pos);
                false
            });
            generating_chunks.retain(|chunk_pos| {
                for (player_chunk, render_distance) in player_positions.iter() {
                    if render_distance.is_chunk_visible(*player_chunk, *chunk_pos) {
                        return true;
                    }
                }
                worldgen_worker.dequeue(*chunk_pos);
                false
            });
            chunk_lighting_updates.retain(|chunk_pos| {
                for (player_chunk, render_distance) in player_positions.iter() {
                    if render_distance.is_chunk_visible(*player_chunk, *chunk_pos) {
                        return true;
                    }
                }
                light.remove(chunk_pos);
                false
            });
            self.timing.record_part("Drop far chunks");
        }

        send_debug_info("Chunks", "server",
                        format!(
                            "Server loaded chunks = {}\nServer loaded light chunks = {}\nServer generating chunks = {}\nServer pending lighting chunks = {}",
                            self.world.chunks.len(),
                            self.world.light.len(),
                            self.generating_chunks.len(),
                            self.chunk_lighting_updates.len(),
                        ));

        // Nothing else to do for now :-)
        send_perf_breakdown("Server", "mainloop", "Server main loop", self.timing.extract_part_averages());
    }
}
