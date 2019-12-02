//! World rendering

use super::buffers::MultiBuffer;
use voxel_rs_common::world::chunk::ChunkPos;
use image::{ImageBuffer, Rgba};
use voxel_rs_common::block::BlockMesh;
use super::init::{load_glsl_shader, create_default_pipeline};
use crate::window::WindowBuffers;
use super::world::meshing_worker::MeshingWorker;
use crate::texture::load_image;
use super::frustum::Frustum;
use voxel_rs_common::debug::send_debug_info;
use voxel_rs_common::world::World;

mod meshing;
mod meshing_worker;

/// All the state necessary to render the world.
pub struct WorldRenderer {
    // Chunk meshing
    meshing_worker: MeshingWorker,
    // View-projection matrix
    uniform_view_proj: wgpu::Buffer,
    // Chunk rendering
    chunk_index_buffers: MultiBuffer<ChunkPos, u32>,
    chunk_vertex_buffers: MultiBuffer<ChunkPos, ChunkVertex>,
    chunk_pipeline: wgpu::RenderPipeline,
    chunk_bind_group: wgpu::BindGroup,
}

impl WorldRenderer {
    pub fn new(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture_atlas: ImageBuffer<Rgba<u8>, Vec<u8>>,
        block_meshes: Vec<BlockMesh>,
    ) -> Self {
        let mut compiler = shaderc::Compiler::new().expect("Failed to create shader compiler");

        // Load texture atlas
        let texture_atlas = load_image(device, encoder, texture_atlas);
        let texture_atlas_view = texture_atlas.create_default_view();

        // Create view projection buffer
        let uniform_view_proj = device.create_buffer(&wgpu::BufferDescriptor {
            size: 64,
            usage: (wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST),
        });

        // Create uniform bind group
        let chunk_bind_group_layout = device.create_bind_group_layout(&CHUNK_BIND_GROUP_LAYOUT);
        let chunk_bind_group = create_chunk_bind_group(
            device,
            &chunk_bind_group_layout,
            &texture_atlas_view,
            &uniform_view_proj
        );

        // Create chunk pipeline
        let chunk_pipeline = {
            let vertex_shader =
                load_glsl_shader(&mut compiler, shaderc::ShaderKind::Vertex, "assets/shaders/world.vert");
            let fragment_shader =
                load_glsl_shader(&mut compiler, shaderc::ShaderKind::Fragment, "assets/shaders/world.frag");

            create_default_pipeline(
                device,
                &chunk_bind_group_layout,
                vertex_shader.as_binary(),
                fragment_shader.as_binary(),
                wgpu::PrimitiveTopology::TriangleList,
                wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<ChunkVertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &CHUNK_VERTEX_ATTRIBUTES,
                },
                false,
            )
        };

        Self {
            meshing_worker: MeshingWorker::new(block_meshes),
            uniform_view_proj,
            chunk_index_buffers: MultiBuffer::with_capacity(device, 1000, wgpu::BufferUsage::INDEX),
            chunk_vertex_buffers: MultiBuffer::with_capacity(device, 1000, wgpu::BufferUsage::VERTEX),
            chunk_pipeline,
            chunk_bind_group,
        }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        buffers: WindowBuffers,
        data: &crate::window::WindowData,
        frustum: &Frustum,
        enable_culling: bool,
    ) {
        //============= RECEIVE CHUNK MESHES =============//
        for (pos, vertices, indices) in self.meshing_worker.get_processed_chunks() {
            if vertices.len() > 0 && indices.len() > 0 {
                self.chunk_vertex_buffers.update(
                    device,
                    encoder,
                    pos,
                    &vertices[..],
                );
                self.chunk_index_buffers.update(
                    device,
                    encoder,
                    pos,
                    &indices[..],
                );
            }
        }

        //============= RENDER =============//
        // TODO: what if win_h is 0 ?
        let aspect_ratio = {
            let winit::dpi::PhysicalSize {
                width: win_w,
                height: win_h,
            } = data.physical_window_size;
            win_w / win_h
        };

        let view_mat = frustum.get_view_matrix();
        let planes = frustum.get_planes(aspect_ratio);
        let view_proj_mat = frustum.get_view_projection(aspect_ratio);
        let opengl_to_wgpu = nalgebra::Matrix4::from([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, -1.0, 0.0, 0.0],
            [0.0, 0.0, 0.5, 0.0],
            [0.0, 0.0, 0.5, 1.0],
        ]);
        let view_proj: [[f32; 4]; 4] = nalgebra::convert::<nalgebra::Matrix4<f64>, nalgebra::Matrix4<f32>>(opengl_to_wgpu * view_proj_mat).into();

        // Update view_proj matrix
        let src_buffer = device
            .create_buffer_mapped(4, wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&view_proj);
        encoder.copy_buffer_to_buffer(&src_buffer, 0, &self.uniform_view_proj, 0, 64);

        // Draw all the chunks
        {
            let mut rpass = super::render::create_default_render_pass(encoder, buffers);
            rpass.set_pipeline(&self.chunk_pipeline);
            rpass.set_bind_group(0, &self.chunk_bind_group, &[]);
            rpass.set_vertex_buffers(0, &[(&self.chunk_vertex_buffers.get_buffer(), 0)]);
            rpass.set_index_buffer(&self.chunk_index_buffers.get_buffer(), 0);
            let mut count = 0;
            for chunk_pos in self.chunk_index_buffers.keys() {
                if !enable_culling || Frustum::contains_chunk(&planes, &view_mat, chunk_pos) {
                    count += 1;
                    let (index_pos, index_len) = self.chunk_index_buffers.get_pos_len(&chunk_pos).unwrap();
                    let (vertex_pos, _) = self.chunk_vertex_buffers.get_pos_len(&chunk_pos).unwrap();
                    rpass.draw_indexed(
                        (index_pos as u32)..((index_pos + index_len) as u32),
                        vertex_pos as i32,
                        0..1,
                    );
                }
            }
            send_debug_info(
                "Render",
                "renderedchunks",
                format!("{} chunks were rendered", count),
            );
        }
    }

    pub fn update_chunk(
        &mut self,
        world: &World,
        pos: ChunkPos,
    ) {
        self.meshing_worker.enqueue_chunk(self::meshing::ChunkMeshData::create_from_world(world, pos));
    }

    pub fn remove_chunk(&mut self, pos: ChunkPos) {
        self.meshing_worker.dequeue_chunk(pos);
        self.chunk_vertex_buffers.remove(&pos);
        self.chunk_index_buffers.remove(&pos);
    }
}

/*========== CHUNK RENDERING ==========*/
/// Chunk vertex
#[derive(Debug, Clone, Copy)]
pub struct ChunkVertex {
    pub pos: [f32; 3],
    pub texture_top_left: [f32; 2],
    pub texture_size: [f32; 2],
    pub texture_max_uv: [f32; 2],
    pub texture_uv: [f32; 2],
    pub occl_and_face: u32,
}

/// Chunk vertex attributes
const CHUNK_VERTEX_ATTRIBUTES: [wgpu::VertexAttributeDescriptor; 6] = [
    wgpu::VertexAttributeDescriptor {
        shader_location: 0,
        format: wgpu::VertexFormat::Float3,
        offset: 0,
    },
    wgpu::VertexAttributeDescriptor {
        shader_location: 1,
        format: wgpu::VertexFormat::Float2,
        offset: 4 * 3,
    },
    wgpu::VertexAttributeDescriptor {
        shader_location: 2,
        format: wgpu::VertexFormat::Float2,
        offset: 4 * (3 + 2),
    },
    wgpu::VertexAttributeDescriptor {
        shader_location: 3,
        format: wgpu::VertexFormat::Float2,
        offset: 4 * (3 + 2 + 2),
    },
    wgpu::VertexAttributeDescriptor {
        shader_location: 4,
        format: wgpu::VertexFormat::Float2,
        offset: 4 * (3 + 2 + 2 + 2),
    },
    wgpu::VertexAttributeDescriptor {
        shader_location: 5,
        format: wgpu::VertexFormat::Uint,
        offset: 4 * (3 + 2 + 2 + 2 + 2),
    },
];

const CHUNK_BIND_GROUP_LAYOUT: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
    bindings: &[
        wgpu::BindGroupLayoutBinding {
            binding: 0,
            visibility: wgpu::ShaderStage::VERTEX,
            ty: wgpu::BindingType::UniformBuffer { dynamic: false },
        },
        wgpu::BindGroupLayoutBinding {
            binding: 1,
            visibility: wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::Sampler,
        },
        wgpu::BindGroupLayoutBinding {
            binding: 2,
            visibility: wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::SampledTexture {
                multisampled: false,
                dimension: wgpu::TextureViewDimension::D2,
            },
        },
    ],
};

/// Create chunk bind group
fn create_chunk_bind_group(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, texture_atlas_view: &wgpu::TextureView, uniform_view_proj: &wgpu::Buffer) -> wgpu::BindGroup {
    // Create texture sampler
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: 5.0,
        compare_function: wgpu::CompareFunction::Always,
    });

    device.create_bind_group( &wgpu::BindGroupDescriptor {
        layout,
        bindings: &[
            wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: uniform_view_proj,
                    range: 0..64,
                },
            },
            wgpu::Binding {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::Binding {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(texture_atlas_view),
            },
        ],
    })
}